#![allow(clippy::disallowed_macros)]

use bevy::{prelude::*, tasks::block_on};
use bevy_widgetry_job::*;
use std::time::{Duration, Instant};

struct ConsumerA;
struct ConsumerB;

fn ensure(app: &mut App) {
    if !app.is_plugin_added::<JobManagerPlugin>() {
        app.add_plugins(JobManagerPlugin);
    }
}

impl Plugin for ConsumerA {
    fn build(&self, app: &mut App) {
        ensure(app);
    }
}
impl Plugin for ConsumerB {
    fn build(&self, app: &mut App) {
        ensure(app);
    }
}

#[test]
fn consumers_share_one_resource_and_preserve_host_configuration_without_winit() {
    for explicit in [false, true] {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let manager = JobManager::with_buffer_capacity(2).unwrap();
        let (id, mut receiver) = manager
            .spawn_io::<(), _, _, String>(JobOptions::default(), |_| async { Ok(()) })
            .unwrap();
        app.insert_resource(manager.clone());
        if explicit {
            app.add_plugins(JobManagerPlugin);
        }
        app.add_plugins((ConsumerA, ConsumerB));
        app.update();
        assert!(app.is_plugin_added::<JobManagerPlugin>());
        assert!(app.world().resource::<JobManager>().status(id).is_ok());
        block_on(receiver.recv()).unwrap();
        let handle = app.world().resource::<JobManager>().clone();
        let status = std::thread::spawn(move || handle.status(id).unwrap())
            .join()
            .unwrap();
        assert_eq!(status.state, JobState::Completed);
        let mut other_app = App::new();
        other_app.add_plugins(JobManagerPlugin);
        assert_eq!(
            other_app.world().resource::<JobManager>().status(id),
            Err(JobAccessError::NotFound)
        );
    }
}

#[derive(Resource)]
struct Inbox {
    receiver: JobReceiver<usize>,
    values: Vec<usize>,
    finished: bool,
}

fn read_one(mut inbox: ResMut<Inbox>) -> Result {
    match inbox.receiver.try_recv() {
        Ok(JobMessage::Item(value)) => inbox.values.push(*value),
        Ok(JobMessage::Finished(_)) => inbox.finished = true,
        Err(TryRecvError::Empty | TryRecvError::Closed) => {}
        Err(error) => return Err(BevyError::error(error)),
    }
    Ok(())
}

#[test]
fn headless_consumer_can_use_a_frame_budget_and_distinguish_execution_from_display_completion() {
    for first in [false, true] {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, JobManagerPlugin));
        let manager = app.world().resource::<JobManager>().clone();
        let (id, receiver) = manager
            .spawn_io::<usize, _, _, SendError>(JobOptions::default(), |context| async move {
                for value in 0..3 {
                    context.send(value).await?;
                }
                Ok(())
            })
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !manager.status(id).unwrap().state.is_finished() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        app.insert_resource(Inbox {
            receiver,
            values: Vec::new(),
            finished: false,
        });
        if first {
            app.add_systems(First, read_one.after(JobManagerSystems::BeginFrame));
        } else {
            app.add_systems(Update, read_one);
        }
        for count in 1..=3 {
            app.update();
            let inbox = app.world().resource::<Inbox>();
            assert_eq!(inbox.values.len(), count);
            assert!(!inbox.finished);
        }
        app.update();
        assert!(app.world().resource::<Inbox>().finished);
    }
}
