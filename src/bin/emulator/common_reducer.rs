use std::time::Instant;

use crate::common_state::{Action, AppState, CommonAction, CommonState, PerformanceData};
use crate::hardware_reducer::reduce_breakpoint_hardware;

pub fn reduce(state: &mut AppState, action: &Action) {
    match action {
        Action::Common(common_action) => match state {
            AppState::Hardware(hardware_state) => reduce_common(hardware_state, common_action),
            AppState::VM(vm_state) => reduce_common(vm_state, common_action),
            AppState::Start => panic!(
                "Received common action {:?} when in state AppState::Start",
                common_action
            ),
        },
        Action::Breakpoint(breakpoint_action) => match state {
            AppState::Hardware(hardware_state) => {
                reduce_breakpoint_hardware(hardware_state, breakpoint_action)
            }
            AppState::VM(vm_state) => todo!(),
            AppState::Start => todo!(),
        },
        Action::Quit => todo!(),
    }
}

pub fn reduce_common(state: &mut impl CommonState, action: &CommonAction) {
    match action {
        CommonAction::StepClicked => {}
        CommonAction::RunClicked => {
            state.shared_state_mut().run_started = true;
        }
        CommonAction::PauseClicked => {
            state.shared_state_mut().run_started = false;
        }
        CommonAction::ResetClicked => {
            state.reset();
            state.shared_state_mut().run_started = false;
        }
        CommonAction::BreakpointsClicked => {
            state.shared_state_mut().breakpoints_open = !state.shared_state().breakpoints_open;
        }
        CommonAction::BreakpointsClosed => {
            state.shared_state_mut().breakpoints_open = false;
        }
        CommonAction::SpeedSliderMoved(new_value) => {
            state.shared_state_mut().desired_steps_per_second = *new_value;
        }
    }
}

pub fn steps_to_run(
    desired_steps_per_second: u64,
    last_frame_time: f32,
    performance_data: &mut PerformanceData,
    state: &impl CommonState,
    action: &Option<Action>,
) -> u64 {
    if !state.shared_state().run_started
        || performance_data.previous_desired_steps_per_second != desired_steps_per_second
    {
        performance_data.run_start = None;
        performance_data.steps_during_last_frame = 0;
        performance_data.total_steps = 0;
        performance_data.previous_desired_steps_per_second = desired_steps_per_second;
    }

    if !state.shared_state().run_started {
        return (action == &Some(Action::Common(CommonAction::StepClicked))) as u64;
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

    return steps_to_run;
}