use bevy::{
    app::MainScheduleOrder, ecs::schedule::{ExecutorKind, ScheduleLabel}, prelude::*
};

pub(super) struct DunwardRenderPlugin;
impl Plugin for DunwardRenderPlugin {
    fn build(&self, app: &mut App) {
        let mut render_schedule = Schedule::new(Render);
        render_schedule.set_executor_kind(ExecutorKind::SingleThreaded);
        app.add_schedule(render_schedule);
        app.world_mut().resource_mut::<MainScheduleOrder>()
            .insert_after(Update, Render);
        
        app.add_systems(Render, hello);
    }
}

#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
struct Render;

fn hello() {
    info!("Render plugin is working!");
}
