use defmt::warn;
use embassy_futures::join;
use embassy_stm32::can::Frame;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, channel::Sender, mutex::Mutex};
use embassy_time::{Duration, Timer};

use crate::{nmt::NmtState, node::Context, object_dictionary::ObjectDictionary};

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ErrorKind {
    ErrorType,
    NoEntry
}

pub struct HeartbeatProducer<'a, 'b, 'c, const N: usize, const R: usize> {
    pub(crate) context: &'c Mutex<ThreadModeRawMutex, Context>,
    pub(crate) object_dictionary: &'a Mutex<ThreadModeRawMutex, ObjectDictionary<N>>,
    pub(crate) can_tx_sender: Sender<'b, ThreadModeRawMutex, embassy_stm32::can::Frame, R>,
}

impl<'a, 'b, 'c, const N: usize, const R: usize> HeartbeatProducer<'a, 'b, 'c, N, R> {
    pub async fn timeout(&self) -> Result<u16, ErrorKind> {
        let locked = self.object_dictionary.lock().await;
        match locked.get_entry(0x1017, 00) {
            Some(value) => match value.value {
                crate::object_dictionary::Value::Uint16(t) => return Ok(t),
                _ => Err(ErrorKind::ErrorType)
            },
            None => Err(ErrorKind::NoEntry),
        }
    }

    pub async fn run(&self, on_error_timeout: Duration) -> ! {
        loop {
            let timeout = match self.timeout().await {
                Ok(t) => t,
                Err(e) => {warn!("HeartbeatProducer: {}", e); 0},
            };
    
            if timeout == 0 {
                Timer::after(on_error_timeout).await;
                continue;
            }

            let node_id;
            let nmt_state;
            {
                let locked_context = self.context.lock().await;
                node_id = locked_context.node_id;
                nmt_state = locked_context.nmt_state;
            }

            let msg = Frame::new_standard(0x700 + node_id as u16, &[nmt_state.into()]).unwrap();

            self.can_tx_sender.send(msg).await;
            Timer::after_millis(timeout as u64).await;
        }
    }
}