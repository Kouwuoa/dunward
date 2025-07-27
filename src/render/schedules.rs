use bevy::{
    app::MainScheduleOrder,
    ecs::schedule::{ExecutorKind, ScheduleLabel},
    prelude::*,
};

#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub(super) struct Render;

pub(super) fn init_schedules(app: &mut App) {
    let mut render_schedule = Schedule::new(Render);
    render_schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    app.add_schedule(render_schedule);
    app.world_mut()
        .resource_mut::<MainScheduleOrder>()
        .insert_after(Update, Render);
}
