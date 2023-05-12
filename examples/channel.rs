use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
};

use lenar::*;

fn main() {
    use parser::*;
    use runtime::*;

    let code = r#"

        let bus = channel();

        let handle = thread(
            fn(bus) {
                println("received" recv(channel));
                println("received" recv(channel));
            }
            bus
        );

        send(bus 1);
        send(bus 1);
        
        println("Finished!");
    "#;

    #[derive(Debug, Clone)]
    pub enum OwnedLenarValue {
        Usize(usize),
        Bool(bool),
    }

    impl<'a> TryFrom<LenarValue<'a>> for OwnedLenarValue {
        type Error = LenarError;

        fn try_from(value: LenarValue<'a>) -> Result<Self, Self::Error> {
            match value {
                LenarValue::Bool(b) => Ok(OwnedLenarValue::Bool(b)),
                LenarValue::Usize(u) => Ok(OwnedLenarValue::Usize(u)),
                _ => Err(LenarError::WrongValue(
                    "This value cannot be shared between threads".to_string(),
                )),
            }
        }
    }

    impl<'a> TryInto<LenarValue<'a>> for OwnedLenarValue {
        type Error = LenarError;

        fn try_into(self) -> Result<LenarValue<'a>, Self::Error> {
            match self {
                OwnedLenarValue::Bool(b) => Ok(LenarValue::Bool(b)),
                OwnedLenarValue::Usize(u) => Ok(LenarValue::Usize(u)),
            }
        }
    }

    type ChannelsPool = Arc<Mutex<Slab<ChannelInstance>>>;

    let channels_pool = ChannelsPool::default();

    #[derive(Debug)]
    struct ChannelFunc {
        channels_pool: ChannelsPool,
    }

    impl ChannelFunc {
        pub fn new(channels_pool: ChannelsPool) -> Self {
            Self { channels_pool }
        }
    }

    impl RuntimeFunction for ChannelFunc {
        fn call<'s>(
            &mut self,
            _args: Vec<LenarValue<'s>>,
            _objects_map: &'s Arc<Parser>,
        ) -> LenarResult<LenarValue<'s>> {
            let channel = ChannelInstance::new();
            let rid = self.channels_pool.lock().unwrap().insert(channel);
            Ok(LenarValue::Usize(rid))
        }

        fn get_name<'s>(&self) -> &'s str {
            "channel"
        }
    }

    #[derive(Debug)]
    struct ChannelInstance {
        sender: Sender<OwnedLenarValue>,
        receiver: Arc<Mutex<Receiver<OwnedLenarValue>>>,
    }

    impl ChannelInstance {
        pub fn new() -> Self {
            let (sender, receiver) = channel();
            Self {
                receiver: Arc::new(Mutex::new(receiver)),
                sender,
            }
        }
    }

    #[derive(Debug)]
    struct SendFunc {
        channels_pool: ChannelsPool,
    }

    impl SendFunc {
        pub fn new(channels_pool: ChannelsPool) -> Self {
            Self { channels_pool }
        }
    }

    impl RuntimeFunction for SendFunc {
        fn call<'s>(
            &mut self,
            mut args: Vec<LenarValue<'s>>,
            _objects_map: &'s Arc<Parser>,
        ) -> LenarResult<LenarValue<'s>> {
            let rid = args
                .remove(0)
                .as_integer()
                .ok_or(LenarError::WrongValue("Expected a channel ID.".to_string()))?;

            let channels_pool = self.channels_pool.lock().unwrap();
            let channel = channels_pool.get(rid).unwrap();

            let message = args.remove(0);
            let message = OwnedLenarValue::try_from(message)?;
            channel
                .sender
                .send(message)
                .map_err(|_| LenarError::WrongValue("Failed sending message.".to_string()))?;

            Ok(LenarValue::Void)
        }

        fn get_name<'s>(&self) -> &'s str {
            "send"
        }
    }

    #[derive(Debug)]
    struct RecvFunc {
        channels_pool: ChannelsPool,
    }

    impl RecvFunc {
        pub fn new(channels_pool: ChannelsPool) -> Self {
            Self { channels_pool }
        }
    }

    impl RuntimeFunction for RecvFunc {
        fn call<'s>(
            &mut self,
            mut args: Vec<LenarValue<'s>>,
            _objects_map: &'s Arc<Parser>,
        ) -> LenarResult<LenarValue<'s>> {
            let rid = args
                .remove(0)
                .as_integer()
                .ok_or(LenarError::WrongValue("Expected a channel ID.".to_string()))?;

            let receiver = {
                let channels_pool = self.channels_pool.lock().unwrap();
                let channel = channels_pool.get(rid).unwrap();
                channel.receiver.clone()
            };

            let message = receiver
                .lock()
                .unwrap()
                .recv()
                .map_err(|_| LenarError::WrongValue("Failed receiving message.".to_string()))?;

            message.try_into()
        }

        fn get_name<'s>(&self) -> &'s str {
            "recv"
        }
    }

    let parser = Parser::new(&code).wrap();

    let mut scope = Scope::default();
    scope.setup_globals();

    scope.add_global_function(ChannelFunc::new(channels_pool.clone()));
    scope.add_global_function(SendFunc::new(channels_pool.clone()));
    scope.add_global_function(RecvFunc::new(channels_pool));

    Runtime::run_with_scope(&mut scope, &parser).unwrap();
}
