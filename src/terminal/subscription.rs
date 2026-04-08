use iced::futures::SinkExt;
use iced::stream;
use iced::Subscription;
use once_cell::sync::OnceCell;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

pub type PtyOutput = (Uuid, Vec<u8>);

type PtyReceiver = mpsc::Receiver<PtyOutput>;

static PTY_RX_SLOT: OnceCell<Mutex<Option<PtyReceiver>>> = OnceCell::new();

fn receiver_slot() -> &'static Mutex<Option<PtyReceiver>> {
    PTY_RX_SLOT.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
pub(crate) fn subscription_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceCell<Mutex<()>> = OnceCell::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub fn install_receiver(rx: PtyReceiver) {
    let mut slot = receiver_slot()
        .lock()
        .expect("pty receiver slot mutex poisoned");
    *slot = Some(rx);
}

async fn wait_for_receiver() -> PtyReceiver {
    loop {
        if let Some(receiver) = receiver_slot()
            .lock()
            .expect("pty receiver slot mutex poisoned")
            .take()
        {
            return receiver;
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

pub fn pty_output_stream() -> impl iced::futures::Stream<Item = PtyOutput> {
    stream::channel(100, |mut output| async move {
        loop {
            let mut receiver = wait_for_receiver().await;

            while let Some(message) = receiver.recv().await {
                if output.send(message).await.is_err() {
                    return;
                }
            }
        }
    })
}

pub fn pty_output_subscription() -> Subscription<PtyOutput> {
    Subscription::run_with_id("pty-output-subscription", pty_output_stream())
}

#[cfg(test)]
mod tests {
    use super::{install_receiver, pty_output_stream, subscription_test_lock};
    use iced::futures::StreamExt;
    use std::time::Duration;
    use tokio::sync::mpsc;
    use uuid::Uuid;

    #[test]
    fn stream_forwards_messages_from_installed_receiver() {
        let _guard = subscription_test_lock().lock().expect("subscription lock");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");

        runtime.block_on(async {
            let (tx, rx) = mpsc::channel(8);
            let terminal_id = Uuid::new_v4();
            install_receiver(rx);

            let mut stream = Box::pin(pty_output_stream());

            tx.send((terminal_id, b"echo hello".to_vec()))
                .await
                .expect("send pty output");

            let received = tokio::time::timeout(Duration::from_millis(250), stream.next())
                .await
                .expect("subscription stream item within timeout")
                .expect("subscription stream output");

            assert_eq!(received.0, terminal_id);
            assert_eq!(received.1, b"echo hello".to_vec());
        });
    }
}
