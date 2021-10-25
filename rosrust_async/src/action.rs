use rosrust::error::Result as RosResult;
use rosrust_actionlib::{ self as actionlib, action_server, Action, ActionGoal, ActionResponse };
use tokio::sync::mpsc;
use tokio::task;

pub struct ActionServer<T: Action> {
    _raii: actionlib::ActionServer<T>,
    rx: mpsc::Receiver<ActionHandle<T>>,
}

pub struct ActionHandle<T: Action> {
    handle: action_server::ServerSimpleGoalHandle<T>,
}

impl<T: Action> ActionServer<T> {
    // TODO: I think this should actually be async as well. Pretty sure it's a
    // blocking operation to call `ActionServer::new_simple`.
    pub fn new(topic: impl AsRef<str>) -> RosResult<Self> {
        // Why 16 of buffer size? Why not!
        let (tx, rx) = mpsc::channel(16);
        let _raii: actionlib::ActionServer<T> = actionlib::ActionServer::new_simple(topic.as_ref(), move |handle| {
            if let Err(_) = tx.blocking_send(ActionHandle { handle }) {
                panic!("unable to send handle");
            }
        })?;

        Ok(ActionServer { _raii, rx })
    }

    pub async fn recv(&mut self) -> ActionHandle<T> {
        self.rx.recv().await.unwrap()
    }
}

#[derive(Debug)]
pub struct PubFeedBackError;
pub type ResponseBuilder<'a, T> = action_server::ServerGoalHandleMessageBuilder<'a, T>;
pub type GoalBody<T> = <<T as Action>::Goal as ActionGoal>::Body;
pub type ActionFeedback<T> = <<T as Action>::Feedback as ActionResponse>::Body;

impl<T: Action> ActionHandle<T> {
    // TODO: This should probably take ownership of self to ensure it's only going to get called once.
    pub fn response_builder(&self) -> ResponseBuilder<'_, T> {
        self.handle.response()
    }

    pub async fn publish_feedback(&self, feedback: ActionFeedback<T>) -> Result<(), PubFeedBackError> {
        // SAFETY: We create a static reference to the handle that can only really live for the lifetime of
        // this function, so it's unsafe. However we know that `&self` must be valid until the end of the
        // function and we are `await`ing the `spawn_blocking` call so we make sure that the spawned thread
        // will terminate **before** this function returns.
        let handle: &'static action_server::ServerSimpleGoalHandle<T> = unsafe {
            std::mem::transmute(&self.handle)
        };
        task::spawn_blocking(move || {
            if handle.handle().publish_feedback(feedback) {
                Ok(())
            } else {
                Err(PubFeedBackError)
            }
        })
        .await
        .unwrap()
    }

    pub fn goal(&self) -> &GoalBody<T> {
        self.handle.goal()
    }

    pub fn goal_id(&self) -> actionlib::GoalID {
        self.handle.handle().goal_id()
    }

    pub fn goal_status(&self) -> actionlib::GoalStatus {
        self.handle.handle().goal_status()
    }

    pub fn canceled(&self) -> bool {
        self.handle.canceled()
    }
}