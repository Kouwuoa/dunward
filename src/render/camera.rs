use std::f32::consts::PI;
use bevy::math::{Vec2, Vec3, Vec4, Mat4};
use bevy::prelude::*;

pub(super) struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera);
    }
}

fn spawn_camera(
    mut commands: Commands,
) {
    commands.spawn((
        Camera::new(),
    ));
}

#[derive(Component)]
    pub(super) struct Camera {
        position: Vec3,
        forward: Vec3,
        up: Vec3,
        right: Vec3,
        world_up: Vec3,
        fov_y_deg: f32,
        near: f32,
        far: f32,
        pivot: Vec3,

        dirty: bool, // Indicates if the camera's view or projection matrix needs to be recalculated
    }

    impl Camera {
        const DEFAULT_FOV_Y_DEG: f32 = 45.0;

        pub fn new() -> Self {
            Self {
                position: Vec3::new(0.0, 0.0, 5.0),
                forward: Vec3::NEG_Z,
                up: Vec3::Y,
                right: Vec3::X,
                world_up: Vec3::Y,
                fov_y_deg: Self::DEFAULT_FOV_Y_DEG,
                near: 0.1,
                far: 100.0,
                pivot: Vec3::ZERO,
                dirty: true,
            }
        }

        pub fn set_position_and_look_at_pivot(&mut self, position: Vec3) {
            self.position = position;
            self.look_at(self.pivot);
        }

        pub fn set_position(&mut self, position: Vec3) {
            if position == self.position {
                return;
            }
            self.position = position;
            self.dirty = true;
        }

        pub fn look_at(&mut self, target: Vec3) {
            if target == self.position {
                return;
            }
            self.pivot = target;
            self.forward = (target - self.position).normalize();
            self.right = self.forward.cross(self.world_up).normalize();
            self.up = self.right.cross(self.forward).normalize();
            self.dirty = true;
        }

        pub fn get_viewproj_mat(
            &self,
            viewport_width: f32,
            viewport_height: f32,
        ) -> Mat4 {
            self.get_proj_mat(viewport_width, viewport_height) * self.get_view_mat()
        }

        pub fn get_view_mat(&self) -> Mat4 {
            Mat4::look_to_rh(self.position, self.forward, self.up)
        }

        pub fn get_proj_mat(
            &self,
            viewport_width: f32,
            viewport_height: f32,
        ) -> Mat4 {
            let aspect_ratio = viewport_width / viewport_height;
            Mat4::perspective_rh(
                self.fov_y_deg.to_radians(),
                aspect_ratio,
                self.near,
                self.far,
            )
        }

        pub fn get_pitch(&self) -> f32 {
            calculate_pitch(self.forward)
        }

        pub fn mouse_zoom(&mut self, mouse_wheel_delta_y: f32)
        {
            let new_pos = self.position + self.forward * mouse_wheel_delta_y;
            if new_pos.distance(self.pivot) > 0.1 {
                self.set_position(new_pos);
            }
        }

        pub fn mouse_rotate(
            &mut self,
            prev_mouse_pos: Vec2,
            curr_mouse_pos: Vec2,
            viewport_width: f32,
            viewport_height: f32,
        )
        {
            // Get the homogeneous positions of the camera eye and pivot
            let mut pos = Vec4::from((self.position, 1.0));
            let piv = Vec4::from((self.pivot, 1.0));

            // Calculate the amount of rotation given the mouse movement
            // Left to right = 2*PI = 360 deg
            let delta_angle_x = 2.0 * PI / viewport_width;
            // Top to bottom = PI = 180 deg
            let mut delta_angle_y = PI / viewport_height;
            let angle_x = (prev_mouse_pos.x - curr_mouse_pos.x) * delta_angle_x;
            let angle_y = (prev_mouse_pos.y - curr_mouse_pos.y) * delta_angle_y;

            // Handle case where the camera's forward is the same as its up
            let cos_angle = self.forward.dot(self.up);
            if cos_angle * delta_angle_y.signum() > 0.99 {
                delta_angle_y = 0.0;
            }

            // Rotate the camera around the pivot point on the up axis
            let rot_x = Mat4::from_axis_angle(self.up, angle_x);
            pos = (rot_x * (pos - piv)) + piv;

            // Rotate the camera around the pivot point on the right axis
            let rot_y = Mat4::from_axis_angle(self.right, angle_y);
            pos = (rot_y * (pos - piv)) + piv;

            // Update camera position
            self.set_position(pos.truncate());
        }
    }

    fn calculate_pitch(forward: Vec3) -> f32 {
        let forward = forward.normalize();
        forward.y.clamp(-1.0, 1.0).asin()
    }

    fn calculate_yaw(forward: Vec3) -> f32 {
        let forward = forward.normalize();
        forward.z.atan2(forward.x)
    }

    fn calculate_roll(forward: Vec3, up: Vec3) -> f32 {
        let right = forward.cross(up).normalize();
        right.y.atan2(right.x)
    }

    fn calculate_direction(pitch: f32, yaw: f32) -> Vec3 {
        Vec3::new(
            yaw.cos() * pitch.cos(),
            pitch.sin(),
            yaw.sin() * pitch.cos(),
        )
    }