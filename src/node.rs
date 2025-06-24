use defmt::{info, warn};
use embassy_futures::{join, select::select};
use embassy_stm32::{can::{CanRx, CanTx, Frame}};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, channel::{Channel, Receiver, Sender}, mutex::Mutex};
use embassy_time::{Timer, Duration};
use embedded_can::StandardId;

use crate::{nmt::{NmtCommand, NmtState}, node, object_dictionary::ObjectDictionary};

pub use crate::heartbeat::HeartbeatProducer;

pub struct NodeReceiver<'b, const R: usize> {
    can_rx: CanRx<'static>,
    can_rx_sender: Sender<'b, ThreadModeRawMutex, embassy_stm32::can::frame::Envelope, R>
}

impl <'b, const R: usize> NodeReceiver<'b, R> {
    pub async fn run(&mut self, timeout_on_can_error: Duration) -> ! {
        loop {
            match self.can_rx.read().await {
                Ok(n) => {
                    let _ = self.can_rx_sender.try_send(n).inspect_err(|e| warn!("Can rx buffer err {}", e));
                }
                Err(e) => {
                    info!("Can bus error: {}", e);
                    Timer::after(timeout_on_can_error).await;
                }
            }
        }
    }
}

pub struct NodeSender<'b, const R: usize> {
    can_tx: CanTx<'static>,
    can_tx_receiver: Receiver<'b, ThreadModeRawMutex, embassy_stm32::can::Frame, R>
}

impl <'b, const R: usize> NodeSender<'b, R> {
    pub async fn run(&mut self, transmit_timeout: Duration) -> ! {
        loop {
            let frame = self.can_tx_receiver.receive().await;

            let timeout = async { Timer::after(transmit_timeout).await };
            let write = self.can_tx.write(&frame);

            match select(write, timeout).await {
                embassy_futures::select::Either::First(_) => (),
                embassy_futures::select::Either::Second(_) => warn!("Can error: transmit timeout"),
            }
        }
    }
}

pub struct Context {
    pub(crate) node_id: u8,
    pub(crate) nmt_state: NmtState,
}

impl Context {
    pub fn new(node_id: u8) -> Self {
        Self {
            node_id,
            nmt_state: NmtState::Initializing
        }
    }
}

pub struct Node<'a, 'b, 'c, const N: usize, const R: usize> {
    context: &'c Mutex<ThreadModeRawMutex, Context>,
    object_dictionary: &'a Mutex<ThreadModeRawMutex, ObjectDictionary<N>>,
    can_rx_receiver: Receiver<'b, ThreadModeRawMutex, embassy_stm32::can::frame::Envelope, R>,
    can_tx_sender: Sender<'b, ThreadModeRawMutex, embassy_stm32::can::Frame, R>,
}

impl<'a, 'b, 'c, const N: usize, const R: usize> Node<'a, 'b, 'c, N, R> {
    pub fn new(
        context: &'c Mutex<ThreadModeRawMutex, Context>,
        object_dictionary: &'a Mutex<ThreadModeRawMutex, ObjectDictionary<N>>, 
        can_tx: CanTx<'static>,
        can_rx: CanRx<'static>,
        can_rx_channel: &'b Channel<ThreadModeRawMutex, embassy_stm32::can::frame::Envelope, R>,
        can_tx_channel: &'b Channel<ThreadModeRawMutex, embassy_stm32::can::Frame, R>
    ) -> (Self, NodeReceiver<'b, R>, NodeSender<'b, R>, HeartbeatProducer<'a, 'b, 'c, N, R>) {
        let receiver = NodeReceiver {
            can_rx,
            can_rx_sender: can_rx_channel.sender(),
        };

        let sender = NodeSender {
            can_tx,
            can_tx_receiver: can_tx_channel.receiver(),
        };

        let heartbeat_producer = HeartbeatProducer {
            context,
            object_dictionary,
            can_tx_sender: can_tx_channel.sender()
        };

        let node = Self {
            object_dictionary,
            context,
            can_rx_receiver: can_rx_channel.receiver(), 
            can_tx_sender: can_tx_channel.sender(), 
        };

        (node, receiver, sender, heartbeat_producer)
    }

    // pub fn node_id(&self) -> u8 {
    //     // self.node_id
    // }

    // pub fn nmt_state(&self) -> NmtState {
    //     // self.nmt_state
    // }

    pub async fn process(&mut self) -> ! {
        loop {
            let n = self.can_rx_receiver.receive().await;

            let frame = n.frame;
            let cob_id = frame.id();

            let node_id;
            {
                let locked_context = self.context.lock().await;
                node_id = locked_context.node_id;
            }
            
            match cob_id {
                // Handle NMT command (COB-ID 0x000)
                embedded_can::Id::Standard(id) if id.as_raw() == 0x000 => {
                    self.process_nmt_command(frame.data()).await;
                }

                // Process PDOs (example: 0x200 - 0x4FF)
                embedded_can::Id::Standard(id) if id.as_raw() >= 0x200 && id.as_raw() <= 0x4FF => {
                    self.process_pdo(frame.data()).await;
                }

                // Handle SDO (COB-ID 0x600-0x67F for requests and 0x580-0x5FF for responses)
                embedded_can::Id::Standard(id) if (id.as_raw() == 0x600 + node_id as u16) => {
                    self.process_sdo_request(frame.data()).await;
                }
                // embedded_can::Id::Standard(id) if (id.as_raw() >= 0x580 && id.as_raw() <= 0x5FF) => {
                //     self.process_sdo_response(frame.data()).await;
                // }

                // Handle SYNC message (COB-ID 0x080)
                embedded_can::Id::Standard(id) if id.as_raw() == 0x080 => {
                    self.process_sync().await;
                }

                // Handle Heartbeat message (COB-ID 0x700 + node_id)
                embedded_can::Id::Standard(id) if id.as_raw() >= 0x700 && id.as_raw() <= StandardId::MAX.as_raw() => {
                    self.process_heartbeat(frame.data()).await;
                }

                // Other messages
                _ => {
                    match cob_id {
                        embedded_can::Id::Standard(id) => {
                            info!("Unhandled frame: COB-ID: {}, data: {}", id.as_raw(), frame.data());
                        }
                        embedded_can::Id::Extended(extended_id) => {
                            info!("Unhandled Extended frame: COB-ID: {}, data: {}", extended_id.as_raw(), frame.data());
                        },
                    }
                }
            }

            Timer::after_millis(1).await;
        }
    }

    // Process NMT Command (COB-ID: 0x000)
    async fn process_nmt_command(&mut self, data: &[u8]) {
        if data.len() < 2 {
            info!("Invalid NMT frame");
            return;
        }

        let command = NmtCommand::from(data[0]); // Assuming `NmtCommand` can be parsed from the first byte
        let received_node_id = data[1];

        {
            let mut locked_context = self.context.lock().await;
            if received_node_id == 0 || received_node_id == locked_context.node_id {
                match command {
                    NmtCommand::EnterOperational => locked_context.nmt_state = NmtState::Operational,
                    NmtCommand::EnterStopped => locked_context.nmt_state = NmtState::Stopped,
                    NmtCommand::EnterPreOperational => locked_context.nmt_state = NmtState::PreOperational,
                    NmtCommand::ResetCommunication => self.reset_communication(),
                    NmtCommand::ResetDevice => self.reset_device(),
                    _ => info!("Unknown NMT command"),
                }

                info!("NMT command processed: {:?}, new state: {:?}", command, locked_context.nmt_state);
            }
        }
    }

    pub async fn heartbear_producer() {

    }

    // Placeholder for processing PDOs (COB-IDs: 0x200 - 0x4FF)
    async fn process_pdo(&self, _data: &[u8]) {
        info!("Processing PDO");
        // PDO logic here
    }

    // Placeholder for processing SDO request (COB-ID: 0x600 - 0x67F)
    async fn process_sdo_request(&self, _data: &[u8]) {
        info!("Processing SDO Request");
        // SDO request logic here
    }

    // Placeholder for processing SDO response (COB-ID: 0x580 - 0x5FF)
    async fn process_sdo_response(&self, _data: &[u8]) {
        info!("Processing SDO Response");
        // SDO response logic here
    }

    // Process SYNC message (COB-ID: 0x080)
    async fn process_sync(&self) {
        info!("Processing SYNC message");
        // SYNC message handling logic here
    }

    // Process Heartbeat message (COB-ID: 0x700 + node_id)
    async fn process_heartbeat(&self, _data: &[u8]) {
        info!("Processing Heartbeat message");
        // Heartbeat message handling logic here
    }

    // Node reset function for NMT ResetNode command
    fn reset_communication(&mut self) {
        // self.nmt_state = NmtState::Initializing;
        // Logic to reset the node state, reinitialize services, etc.
        info!("Node reset");
    }

    // Node reset function for NMT ResetNode command
    fn reset_device(&mut self) {
        // self.nmt_state = NmtState::Initializing;
        // Logic to reset the node state, reinitialize services, etc.
        info!("Node reset");
    }
}

