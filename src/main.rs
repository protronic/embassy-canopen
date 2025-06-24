#![no_std]
#![no_main]

use defmt::*;
use embassy_canopen::node::{Context, HeartbeatProducer, Node, NodeReceiver, NodeSender};
use embassy_canopen::object_dictionary::ObjectDictionary;
use embassy_executor::Spawner;
use embassy_stm32::can::filter::Mask32;
use embassy_stm32::can::{
    Can, CanRx, CanTx, Fifo, Frame, Rx0InterruptHandler, Rx1InterruptHandler, SceInterruptHandler, StandardId, TxInterruptHandler
};
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::peripherals::{self, CAN};
use embassy_stm32::{bind_interrupts, usart};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_sync::pubsub::{PubSubBehavior, PubSubChannel};
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker, Timer};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USB_LP_CAN_RX0 => Rx0InterruptHandler<CAN>;
    CAN_RX1 => Rx1InterruptHandler<CAN>;
    CAN_SCE => SceInterruptHandler<CAN>;
    USB_HP_CAN_TX => TxInterruptHandler<CAN>;
});


static OBJECT_DICTIONARY: StaticCell<Mutex<ThreadModeRawMutex, ObjectDictionary<32>>> = StaticCell::new();
static CAN_RX_CHANNEL: Channel<ThreadModeRawMutex, embassy_stm32::can::frame::Envelope, 10> = Channel::new();
static CAN_TX_CHANNEL: Channel<ThreadModeRawMutex, embassy_stm32::can::Frame, 10> = Channel::new();
static CONTEXT: StaticCell<Mutex<ThreadModeRawMutex, Context>> = StaticCell::new();

#[embassy_executor::task]
async fn node_receiver_task(mut receiver: NodeReceiver<'static, 10>) -> ! {
    receiver.run(Duration::from_secs(5)).await
}

#[embassy_executor::task]
async fn node_sender_task(mut sender: NodeSender<'static, 10>) -> ! {
    sender.run(Duration::from_secs(1)).await
}

#[embassy_executor::task]
async fn node_heartbeat_producer_task(producer: HeartbeatProducer<'static, 'static, 'static, 32, 10>) -> ! {
    producer.run(Duration::from_secs(5)).await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let p = embassy_stm32::init(Default::default());

    // let mut leds: [Output; 8] = [
    //     Output::new(p.PE8, Level::Low, Speed::Medium),
    //     Output::new(p.PE9, Level::Low, Speed::Medium),
    //     Output::new(p.PE10, Level::Low, Speed::Medium),
    //     Output::new(p.PE11, Level::Low, Speed::Medium),
    //     Output::new(p.PE12, Level::Low, Speed::Medium),
    //     Output::new(p.PE13, Level::Low, Speed::Medium),
    //     Output::new(p.PE14, Level::Low, Speed::Medium),
    //     Output::new(p.PE15, Level::Low, Speed::Medium),
    // ];
    info!("Hello mir!");

    let mut can = Can::new(p.CAN, p.PD0, p.PD1, Irqs);
    can.modify_filters()
        .enable_bank(0, Fifo::Fifo0, Mask32::accept_all());
    can.set_bitrate(500_000);
    can.enable().await;
    let (can_tx, can_rx) = can.split();
    
    let od = OBJECT_DICTIONARY.init(Mutex::new(ObjectDictionary::new_canopen_301(Default::default())));
    let ctx = CONTEXT.init(Mutex::new(Context::new(7)));
    let (mut node, node_receiver, node_sender, heartbeat_producer) = Node::new(ctx, od, can_tx, can_rx, &CAN_RX_CHANNEL, &CAN_TX_CHANNEL);

    spawner.spawn(node_receiver_task(node_receiver)).unwrap();
    spawner.spawn(node_sender_task(node_sender)).unwrap();
    spawner.spawn(node_heartbeat_producer_task(heartbeat_producer)).unwrap();
    node.process().await
}