use crate::BROADCAST_CHANNEL;

pub(crate) async fn database_listen_forever() {
    let receiver = &mut BROADCAST_CHANNEL.subscribe();

    loop {
        match receiver.recv().await {
            Ok(_) => todo!(),
            Err(_) => todo!(),
        }
        // ...
    }
}

#[derive(Clone)]
pub(crate) enum ClientEvent {}
