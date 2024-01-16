use std::fs;
use super::instant::Instant;

use super::common_state::{
    Action, AppState, CommonAction, CommonState, PerformanceData, SharedState,
};
use super::hardware_reducer::reduce_breakpoint_hardware;
use super::hardware_state::HardwareState;
use super::vm_reducer::reduce_vm_file_selected;
use super::vm_state::VMState;
use super::EmulatorApp;

pub fn reduce(app: &mut EmulatorApp, action: &Action) {
    match action {
        Action::Common(common_action) => match &mut app.state {
            AppState::Hardware(hardware_state) => {
                reduce_common(hardware_state, &mut app.shared_state, common_action)
            }
            AppState::VM(vm_state) => reduce_common(vm_state, &mut app.shared_state, common_action),
            AppState::Start => panic!(
                "Received common action {:?} when in state AppState::Start",
                common_action
            ),
        },
        Action::Breakpoint(breakpoint_action) => match &mut app.state {
            AppState::Hardware(hardware_state) => {
                reduce_breakpoint_hardware(hardware_state, breakpoint_action)
            }
            AppState::VM(_) => todo!(),
            AppState::Start => todo!(),
        },
        Action::FolderPicked(path) => {
            app.state = AppState::VM(VMState::from_dir(path));
            app.shared_state = Default::default();
        }
        Action::FilePicked(path) => {
            app.state = AppState::Hardware(HardwareState::from_file_contents(&fs::read_to_string(path).unwrap()));
            app.shared_state = Default::default();
        }
        Action::FilesDropped(dropped_files) =>  {
            app.state = AppState::Hardware(HardwareState::from_file_contents(&fs::read_to_string(dropped_files[0].clone().path.unwrap()).unwrap()));
            // let bytes = dropped_files[0].bytes.clone().unwrap().as_ref().to_vec();
            // app.state = AppState::Hardware(HardwareState::from_file_contents(&String::from_utf8(bytes).unwrap()));
            app.shared_state = Default::default();
        }
        Action::Quit => todo!(),
        Action::VMFileSelected(file) => match &mut app.state {
            AppState::Hardware(_) => {
                panic!("Received action {:?} when in state AppState::Start", action)
            }
            AppState::VM(vm_state) => reduce_vm_file_selected(vm_state, file),
            AppState::Start => todo!(),
        },
    }
}

pub fn reduce_common(
    state: &mut impl CommonState,
    shared_state: &mut SharedState,
    action: &CommonAction,
) {
    match action {
        CommonAction::StepClicked => {}
        CommonAction::RunClicked => {
            shared_state.run_started = true;
        }
        CommonAction::PauseClicked => {
            shared_state.run_started = false;
        }
        CommonAction::ResetClicked => {
            state.reset();
            shared_state.run_started = false;
        }
        CommonAction::BreakpointsClicked => {
            shared_state.breakpoints_open = !shared_state.breakpoints_open;
        }
        CommonAction::BreakpointsClosed => {
            shared_state.breakpoints_open = false;
        }
        CommonAction::SpeedSliderMoved(new_value) => {
            shared_state.desired_steps_per_second = *new_value;
        }
    }
}

pub fn steps_to_run(
    desired_steps_per_second: u64,
    last_frame_time: f32,
    performance_data: &mut PerformanceData,
    run_started: bool,
    action: &Option<Action>,
) -> u64 {
    if !run_started
        || performance_data.previous_desired_steps_per_second != desired_steps_per_second
    {
        performance_data.run_start = None;
        performance_data.steps_during_last_frame = 0;
        performance_data.total_steps = 0;
        performance_data.previous_desired_steps_per_second = desired_steps_per_second;
    }

    if !run_started {
        return matches!(action, Some(Action::Common(CommonAction::StepClicked))) as u64;
    }

    let run_start = performance_data.run_start.get_or_insert(Instant::now());

    let run_time = (Instant::now() - *run_start).as_secs_f64();
    let wanted_steps = (desired_steps_per_second as f64 * run_time) as u64;
    let mut steps_to_run = wanted_steps - performance_data.total_steps;

    if performance_data.steps_during_last_frame > 0 {
        steps_to_run = u64::min(
            steps_to_run,
            ((performance_data.steps_during_last_frame as f64) / (last_frame_time as f64 * 60.0))
                as u64,
        );
    }

    performance_data.steps_during_last_frame = steps_to_run;
    performance_data.total_steps += steps_to_run;

    steps_to_run
}
