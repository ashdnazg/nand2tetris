use hashbrown::HashMap;

use crate::{
    hardware::RAM,
    vm::{PushSegment, RunState},
};

pub struct OS {
    memory: Memory,
    screen: Screen,
}

impl Default for OS {
    fn default() -> Self {
        Self {
            memory: Memory::new(0x0800, RAM::SCREEN - 0x0800),
            screen: Screen { color: true },
        }
    }
}

type Func = fn(&mut RunState) -> i16;

impl RunState {
    pub fn call_os(&mut self, function_name: &str) -> bool {
        let function = match function_name {
            "Math.init" => Self::noop,
            "Math.multiply" => Self::math_multiply,
            "Math.divide" => Self::math_divide,
            "Math.min" => Self::math_min,
            "Math.max" => Self::math_max,
            "Math.sqrt" => Self::math_sqrt,
            "Math.abs" => Self::math_abs,
            "Array.new" => Self::memory_alloc,
            "Array.dispose" => Self::memory_dealloc,
            "Keyboard.keyPressed" => Self::keyboard_key_pressed,
            "Screen.init" => Self::noop,
            "Screen.clearScreen" => Self::screen_clear_screen,
            "Screen.setColor" => Self::screen_set_color,
            "Screen.drawPixel" => Self::screen_draw_pixel,
            "Screen.drawLine" => Self::screen_draw_line,
            "Screen.drawRectangle" => Self::screen_draw_rectangle,
            "Screen.drawCircle" => Self::screen_draw_circle,
            "Memory.init" => Self::noop,
            "Memory.peek" => Self::memory_peek,
            "Memory.poke" => Self::memory_poke,
            "Memory.alloc" => Self::memory_alloc,
            "Memory.deAlloc" => Self::memory_dealloc,
            "String.new" => Self::string_new,
            "String.dispose" => Self::memory_dealloc,
            "String.length" => Self::string_length,
            "String.charAt" => Self::string_char_at,
            "String.setCharAt" => Self::string_set_char_at,
            "String.appendChar" => Self::string_append_char,
            "String.eraseLastChar" => Self::string_erase_last_char,
            "String.intValue" => Self::string_int_value,
            "String.setInt" => Self::string_set_int,
            "String.backSpace" => Self::string_backspace,
            "String.doubleQuote" => Self::string_double_quote,
            "String.newLine" => Self::string_new_line,
            "Sys.error" => {
                panic!()
            }
            _ => return false,
        };

        self.call(function);
        true
    }

    fn call(&mut self, f: Func) {
        let return_value = f(self);
        self.ram.push(return_value);
    }

    fn noop(&mut self) -> i16 {
        0
    }

    fn math_multiply(&mut self) -> i16 {
        let x = self.ram.get(0, PushSegment::Argument, 0);
        let y = self.ram.get(0, PushSegment::Argument, 1);

        x * y
    }

    fn math_divide(&mut self) -> i16 {
        let x = self.ram.get(0, PushSegment::Argument, 0);
        let y = self.ram.get(0, PushSegment::Argument, 1);

        if y == 0 {
            panic!()
        }

        x / y
    }

    fn math_min(&mut self) -> i16 {
        let x = self.ram.get(0, PushSegment::Argument, 0);
        let y = self.ram.get(0, PushSegment::Argument, 1);

        x.min(y)
    }

    fn math_max(&mut self) -> i16 {
        let x = self.ram.get(0, PushSegment::Argument, 0);
        let y = self.ram.get(0, PushSegment::Argument, 1);

        x.max(y)
    }

    fn math_sqrt(&mut self) -> i16 {
        let x = self.ram.get(0, PushSegment::Argument, 0);

        if x < 0 {
            panic!();
        }

        (x as f64).sqrt().floor() as i16
    }

    fn math_abs(&mut self) -> i16 {
        let x = self.ram.get(0, PushSegment::Argument, 0);
        x.abs()
    }

    fn screen_clear_screen(&mut self) -> i16 {
        self.ram.contents[(RAM::SCREEN as usize)..(RAM::KBD as usize)].fill(0);

        0
    }

    fn screen_set_color(&mut self) -> i16 {
        self.os.screen.color = self.ram.get(0, PushSegment::Argument, 0) != 0;

        0
    }

    fn screen_draw_pixel(&mut self) -> i16 {
        let x = self.ram.get(0, PushSegment::Argument, 0);
        let y = self.ram.get(0, PushSegment::Argument, 1);
        self.ram.set_pixel(x, y, self.os.screen.color);

        0
    }

    fn screen_draw_line(&mut self) -> i16 {
        let x1 = self.ram.get(0, PushSegment::Argument, 0);
        let y1 = self.ram.get(0, PushSegment::Argument, 1);
        let x2 = self.ram.get(0, PushSegment::Argument, 2);
        let y2 = self.ram.get(0, PushSegment::Argument, 3);

        let dx = (x2 - x1).abs();
        let sx = (x2 - x1).signum();
        let dy = -(y2 - y1).abs();
        let sy = (y2 - y1).signum();
        let mut error = dx + dy;

        let mut x = x1;
        let mut y = y1;

        loop {
            self.ram.set_pixel(x, y, self.os.screen.color);
            if x == x2 && y == y2 {
                break;
            }
            let e2 = 2 * error;
            if e2 >= dy {
                if x2 == x1 {
                    break;
                }
                error += dy;
                x += sx;
            }
            if e2 <= dx {
                if y2 == y1 {
                    break;
                }
                error += dx;
                y += sy
            }
        }
        0
    }

    fn screen_draw_rectangle(&mut self) -> i16 {
        let x1 = self.ram.get(0, PushSegment::Argument, 0);
        let y1 = self.ram.get(0, PushSegment::Argument, 1);
        let x2 = self.ram.get(0, PushSegment::Argument, 2);
        let y2 = self.ram.get(0, PushSegment::Argument, 3);

        for y in y1..y2 {
            for x in x1..=x2 {
                self.ram.set_pixel(x, y, self.os.screen.color);
            }
        }

        0
    }

    fn screen_draw_circle(&mut self) -> i16 {
        let center_x = self.ram.get(0, PushSegment::Argument, 0);
        let center_y = self.ram.get(0, PushSegment::Argument, 1);
        let radius = self.ram.get(0, PushSegment::Argument, 2);
        let r2 = radius * radius;
        for y in (center_y - radius)..=(center_y + radius) {
            let y2 = (y - center_y).abs() * (y - center_y).abs();
            let x_dist = ((r2 - y2).abs() as f64).sqrt().floor() as i16;
            for x in (center_x - x_dist)..=(center_x + x_dist) {
                self.ram.set_pixel(x, y, self.os.screen.color);
            }
        }

        0
    }

    fn keyboard_key_pressed(&mut self) -> i16 {
        self.ram[RAM::KBD]
    }

    fn memory_peek(&mut self) -> i16 {
        let address = self.ram.get(0, PushSegment::Argument, 0);

        self.ram[address]
    }

    fn memory_poke(&mut self) -> i16 {
        let address = self.ram.get(0, PushSegment::Argument, 0);
        let value = self.ram.get(0, PushSegment::Argument, 1);

        self.ram[address] = value;

        0
    }

    fn memory_alloc(&mut self) -> i16 {
        let size = self.ram.get(0, PushSegment::Argument, 0);
        self.os.memory.alloc(size).unwrap()
    }

    fn memory_dealloc(&mut self) -> i16 {
        let object = self.ram.get(0, PushSegment::Argument, 0);
        if self.os.memory.dealloc(object) {
            0
        } else {
            panic!()
        }
    }

    fn string_new(&mut self) -> i16 {
        let initial_capacity = self.ram.get(0, PushSegment::Argument, 0);
        if let Some(s) = VMString::new(self, initial_capacity) {
            s.address
        } else {
            panic!()
        }
    }

    fn string_length(&mut self) -> i16 {
        let address = self.ram.get(0, PushSegment::Argument, 0);
        VMString { address }.length(self)
    }

    fn string_char_at(&mut self) -> i16 {
        let address = self.ram.get(0, PushSegment::Argument, 0);
        let index = self.ram.get(0, PushSegment::Argument, 1);
        VMString { address }.char_at(self, index).unwrap()
    }

    fn string_set_char_at(&mut self) -> i16 {
        let address = self.ram.get(0, PushSegment::Argument, 0);
        let index = self.ram.get(0, PushSegment::Argument, 1);
        let new_value = self.ram.get(0, PushSegment::Argument, 2);
        let result = VMString { address }.set_char_at(self, index, new_value);
        if result.is_some() {
            0
        } else {
            panic!()
        }
    }

    fn string_append_char(&mut self) -> i16 {
        let address = self.ram.get(0, PushSegment::Argument, 0);
        let new_char = self.ram.get(0, PushSegment::Argument, 1);
        let result = VMString { address }.append_char(self, new_char);

        if result.is_some() {
            address
        } else {
            panic!()
        }
    }

    fn string_erase_last_char(&mut self) -> i16 {
        let address = self.ram.get(0, PushSegment::Argument, 0);
        let result = VMString { address }.erase_last_char(self);

        if result.is_some() {
            0
        } else {
            panic!()
        }
    }

    fn string_int_value(&mut self) -> i16 {
        let address = self.ram.get(0, PushSegment::Argument, 0);
        VMString { address }.int_value(self).unwrap()
    }

    fn string_set_int(&mut self) -> i16 {
        let address = self.ram.get(0, PushSegment::Argument, 0);
        let value = self.ram.get(0, PushSegment::Argument, 1);
        let result = VMString { address }.set_int(self, value);

        if result.is_some() {
            0
        } else {
            panic!()
        }
    }

    fn string_backspace(&mut self) -> i16 {
        129
    }

    fn string_double_quote(&mut self) -> i16 {
        34
    }

    fn string_new_line(&mut self) -> i16 {
        128
    }
}

struct VMString {
    address: i16,
}

impl VMString {
    fn new(run_state: &mut RunState, capacity: i16) -> Option<Self> {
        let Some(address) = run_state.os.memory.alloc(2 + capacity) else {
            return None;
        };

        let instance = Self { address };

        *instance.length_mut(run_state) = 0;
        *instance.capacity_mut(run_state) = capacity;

        Some(instance)
    }

    fn char_at(&self, run_state: &RunState, index: i16) -> Option<i16> {
        if self.length(run_state) <= index {
            return None;
        }

        Some(run_state.ram[self.address + 2 + index])
    }

    fn set_char_at(&self, run_state: &mut RunState, index: i16, new_value: i16) -> Option<()> {
        if self.length(run_state) <= index {
            return None;
        }

        run_state.ram[self.address + 2 + index] = new_value;

        Some(())
    }

    fn append_char(&self, run_state: &mut RunState, new_char: i16) -> Option<()> {
        let old_length = self.length(run_state);
        if old_length >= self.capacity(run_state) {
            return None;
        }

        *self.length_mut(run_state) += 1;
        run_state.ram[self.address + 2 + old_length] = new_char;

        Some(())
    }

    fn erase_last_char(&self, run_state: &mut RunState) -> Option<()> {
        if self.length(run_state) <= 0 {
            return None;
        }

        *self.length_mut(run_state) -= 1;
        Some(())
    }

    fn int_value(&self, run_state: &RunState) -> Option<i16> {
        if self.length(run_state) <= 0 {
            return None;
        }

        let mut start = self.address + 2;
        let is_negative = run_state.ram[start] == b'-' as i16;
        if is_negative {
            start += 1;
        }
        let value = run_state.ram.contents
            [start as usize..(self.address + 2 + self.length(run_state)) as usize]
            .iter()
            .try_fold(0, |acc, &i| {
                (i as u8 as char).to_digit(10).map(|d| acc * 10 + d as i32)
            });

        value.map(|v| if is_negative { -v } else { v } as i16)
    }

    fn set_int(&self, run_state: &mut RunState, value: i16) -> Option<()> {
        let mut buffer = [0i16; 6];
        let mut index = 0;
        let mut remainder = (value as i32).abs();
        loop {
            buffer[index] = char::from_digit((remainder % 10) as u32, 10).unwrap() as u8 as i16;
            remainder /= 10;
            index += 1;
            if remainder == 0 {
                break;
            }
        }

        if value < 0 {
            buffer[index] = b'-' as i16;
            index += 1;
        }

        if index > self.capacity(run_state) as usize {
            return None;
        }

        for i in 0..index {
            run_state.ram[self.address + 2 + i as i16] = buffer[index - i - 1];
        }
        *self.length_mut(run_state) = index as i16;

        Some(())
    }

    fn length(&self, run_state: &RunState) -> i16 {
        run_state.ram[self.address]
    }

    fn length_mut<'a>(&self, run_state: &'a mut RunState) -> &'a mut i16 {
        &mut run_state.ram[self.address]
    }

    fn capacity(&self, run_state: &RunState) -> i16 {
        run_state.ram[self.address + 1]
    }

    fn capacity_mut<'a>(&self, run_state: &'a mut RunState) -> &'a mut i16 {
        &mut run_state.ram[self.address + 1]
    }
}

struct Memory {
    hole_starts: HashMap<i16, i16>,
    hole_ends: HashMap<i16, i16>,
    allocs: HashMap<i16, i16>,
}

struct Screen {
    color: bool,
}

impl Memory {
    fn new(start_address: i16, size: i16) -> Self {
        Memory {
            hole_starts: HashMap::from_iter([(start_address, size)]),
            hole_ends: HashMap::from_iter([(start_address + size, size)]),
            allocs: HashMap::new(),
        }
    }

    fn alloc(&mut self, size: i16) -> Option<i16> {
        let Some((&hole_start, &hole_size)) = self
            .hole_starts
            .iter()
            .find(|&(_, hole_size)| *hole_size >= size)
        else {
            return None;
        };

        self.hole_starts.remove(&hole_start);
        self.hole_ends.remove(&(hole_start + hole_size));
        self.allocs.insert(hole_start, size);

        if hole_size != size {
            self.hole_starts.insert(hole_start + size, hole_size - size);
            self.hole_ends
                .insert(hole_start + hole_size, hole_size - size);
        }

        Some(hole_start)
    }

    fn dealloc(&mut self, address: i16) -> bool {
        let Some(size) = self.allocs.remove(&address) else {
            return false;
        };

        let mut hole_start = address;
        let mut hole_size = size;
        if let Some(preceding_size) = self.hole_ends.remove(&address) {
            hole_start -= preceding_size;
            hole_size += preceding_size;

            self.hole_starts.remove(&hole_start);
        }

        if let Some(following_size) = self.hole_starts.remove(&(address + size)) {
            hole_size += following_size;

            self.hole_ends.remove(&(hole_start + hole_size));
        }

        self.hole_starts.insert(hole_start, hole_size);
        self.hole_ends.insert(hole_start + hole_size, hole_size);

        true
    }
}

#[cfg(test)]
mod tests {
    use crate::vm::{PopSegment, Register};

    use super::*;

    impl RunState {
        fn test_instance() -> Self {
            let mut instance = Self {
                current_file_index: 0,
                current_command_index: 0,
                ram: Default::default(),
                os: Default::default(),
                call_stack: vec![],
            };

            instance.ram[Register::ARG] = 100;

            instance
        }
    }

    #[test]
    fn test_string() {
        let mut run_state = RunState::test_instance();
        run_state.ram.set(0, PopSegment::Argument, 0, 10);
        let s = run_state.string_new();
        assert!(s.is_positive());

        run_state.ram.set(0, PopSegment::Argument, 0, s);
        assert_eq!(run_state.string_length(), 0);

        run_state
            .ram
            .set(0, PopSegment::Argument, 1, '5' as u8 as i16);
        assert_eq!(run_state.string_append_char(), s);
        assert_eq!(run_state.string_length(), 1);

        run_state.ram.set(0, PopSegment::Argument, 1, 0);
        assert_eq!(run_state.string_char_at(), '5' as u8 as i16);
        assert_eq!(run_state.string_int_value(), 5);

        run_state
            .ram
            .set(0, PopSegment::Argument, 2, '9' as u8 as i16);
        assert_eq!(run_state.string_set_char_at(), 0);
        assert_eq!(run_state.string_char_at(), '9' as u8 as i16);
        assert_eq!(run_state.string_int_value(), 9);

        run_state.ram.set(0, PopSegment::Argument, 1, i16::MAX);
        assert_eq!(run_state.string_set_int(), 0);
        assert_eq!(run_state.string_length(), 5);
        assert_eq!(run_state.string_int_value(), i16::MAX);

        run_state.ram.set(0, PopSegment::Argument, 1, i16::MIN);
        assert_eq!(run_state.string_set_int(), 0);
        assert_eq!(run_state.string_length(), 6);
        assert_eq!(run_state.string_int_value(), i16::MIN);
    }
}
