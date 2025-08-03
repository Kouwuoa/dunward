use bevy::{
    app::MainScheduleOrder,
    ecs::schedule::{ExecutorKind, ScheduleLabel},
    prelude::*,
};

pub(super) struct SchedulesPlugin;
impl Plugin for SchedulesPlugin {
    fn build(&self, app: &mut App) {
        let mut render_schedule = Schedule::new(Render);
        render_schedule.set_executor_kind(ExecutorKind::SingleThreaded);
        app.add_schedule(render_schedule);
        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_after(PostUpdate, Render);
    }
}

#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub(super) struct Render;

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;

    #[test]
    fn schedules_plugin_adds_render_schedule() {
        let mut app = App::new();
        app.add_plugins(SchedulesPlugin);

        let schedules = app.world().resource::<Schedules>();
        assert!(schedules.contains(Render));
    }

    #[test]
    fn render_schedule_is_single_threaded() {
        let mut app = App::new();
        app.add_plugins(SchedulesPlugin);

        let schedules = app.world().resource::<Schedules>();
        let render_schedule = schedules.get(Render).unwrap();
        assert_eq!(
            render_schedule.get_executor_kind(),
            ExecutorKind::SingleThreaded
        );
    }

    #[test]
    fn render_schedule_is_inserted_after_post_update() {
        let mut app = App::new();
        app.add_plugins(SchedulesPlugin);

        let schedules = app.world().resource::<Schedules>();
        let render_schedule = schedules.get(Render).unwrap();

        // Check that the MainScheduleOrder contains the Render schedule
        {
            let main_schedule_order = app.world().resource::<MainScheduleOrder>();
            assert!(
                main_schedule_order
                    .labels
                    .contains(&render_schedule.label())
            );
        }

        // Add a resource to check the order of schedule execution
        #[derive(Resource)]
        struct TestResource(i32);
        app.insert_resource(TestResource(-1));
        app.add_systems(
            PostUpdate,
            |mut test_res: ResMut<TestResource>| {
                test_res.0 = 1;
            },
        );

        // Add a system to the Render schedule that checks whether Render ran after PostUpdate
        app.add_systems(
            Render,
            |mut test_res: ResMut<TestResource>| {
                test_res.0 = 2;
            },
        );

        // Run the app to execute the schedules
        app.update();

        // Check that the Render schedule ran after PostUpdate
        assert_eq!(app.world().resource::<TestResource>().0, 2);
    }
}
