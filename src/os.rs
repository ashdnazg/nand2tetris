use hashbrown::HashMap;

use crate::{
    characters::character_bitmaps,
    hardware::{RAM, Word},
    vm::{PushSegment, RunState},
};

#[derive(Clone)]
pub struct OS {
    memory: Memory,
    screen: Screen,
    output: Output,
}

impl Default for OS {
    fn default() -> Self {
        Self {
            memory: Memory::new(0x0800, RAM::SCREEN - 0x0800),
            screen: Screen { color: true },
            output: Output { row: 0, col: 0 },
        }
    }
}

type Func = fn(&mut RunState) -> Word;

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
            "Output.init" => Self::noop,
            "Output.moveCursor" => Self::output_move_cursor,
            "Output.printChar" => Self::output_print_char,
            "Output.printString" => Self::output_print_string,
            "Output.printInt" => Self::output_print_int,
            "Output.println" => Self::output_println,
            "Output.backSpace" => Self::output_backspace,
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

    fn noop(&mut self) -> Word {
        0
    }

    fn math_multiply(&mut self) -> Word {
        let x = self.ram.get(0, PushSegment::Argument, 0);
        let y = self.ram.get(0, PushSegment::Argument, 1);

        x * y
    }

    fn math_divide(&mut self) -> Word {
        let x = self.ram.get(0, PushSegment::Argument, 0);
        let y = self.ram.get(0, PushSegment::Argument, 1);

        if y == 0 {
            panic!()
        }

        x / y
    }

    fn math_min(&mut self) -> Word {
        let x = self.ram.get(0, PushSegment::Argument, 0);
        let y = self.ram.get(0, PushSegment::Argument, 1);

        x.min(y)
    }

    fn math_max(&mut self) -> Word {
        let x = self.ram.get(0, PushSegment::Argument, 0);
        let y = self.ram.get(0, PushSegment::Argument, 1);

        x.max(y)
    }

    fn math_sqrt(&mut self) -> Word {
        let x = self.ram.get(0, PushSegment::Argument, 0);

        if x < 0 {
            panic!();
        }

        (x as f64).sqrt().floor() as Word
    }

    fn math_abs(&mut self) -> Word {
        let x = self.ram.get(0, PushSegment::Argument, 0);
        x.abs()
    }

    fn screen_clear_screen(&mut self) -> Word {
        self.ram.contents[(RAM::SCREEN as usize)..(RAM::KBD as usize)].fill(0);

        0
    }

    fn screen_set_color(&mut self) -> Word {
        self.os.screen.color = self.ram.get(0, PushSegment::Argument, 0) != 0;

        0
    }

    fn screen_draw_pixel(&mut self) -> Word {
        let x = self.ram.get(0, PushSegment::Argument, 0);
        let y = self.ram.get(0, PushSegment::Argument, 1);
        self.ram.set_pixel(x, y, self.os.screen.color);

        0
    }

    fn screen_draw_line(&mut self) -> Word {
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

    fn screen_draw_rectangle(&mut self) -> Word {
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

    fn screen_draw_circle(&mut self) -> Word {
        let center_x = self.ram.get(0, PushSegment::Argument, 0);
        let center_y = self.ram.get(0, PushSegment::Argument, 1);
        let radius = self.ram.get(0, PushSegment::Argument, 2);
        let r2 = radius * radius;
        for y in (center_y - radius)..=(center_y + radius) {
            let y2 = (y - center_y).abs() * (y - center_y).abs();
            let x_dist = ((r2 - y2).abs() as f64).sqrt().floor() as Word;
            for x in (center_x - x_dist)..=(center_x + x_dist) {
                self.ram.set_pixel(x, y, self.os.screen.color);
            }
        }

        0
    }

    fn keyboard_key_pressed(&mut self) -> Word {
        self.ram[RAM::KBD]
    }

    fn memory_peek(&mut self) -> Word {
        let address = self.ram.get(0, PushSegment::Argument, 0);

        self.ram[address]
    }

    fn memory_poke(&mut self) -> Word {
        let address = self.ram.get(0, PushSegment::Argument, 0);
        let value = self.ram.get(0, PushSegment::Argument, 1);

        self.ram[address] = value;

        0
    }

    fn memory_alloc(&mut self) -> Word {
        let size = self.ram.get(0, PushSegment::Argument, 0);
        self.os.memory.alloc(size).unwrap()
    }

    fn memory_dealloc(&mut self) -> Word {
        let object = self.ram.get(0, PushSegment::Argument, 0);
        if self.os.memory.dealloc(object) {
            0
        } else {
            panic!()
        }
    }

    fn string_new(&mut self) -> Word {
        let initial_capacity = self.ram.get(0, PushSegment::Argument, 0);
        if let Some(s) = VMString::new(self, initial_capacity) {
            s.address
        } else {
            panic!()
        }
    }

    fn string_length(&mut self) -> Word {
        let address = self.ram.get(0, PushSegment::Argument, 0);
        VMString { address }.length(self)
    }

    fn string_char_at(&mut self) -> Word {
        let address = self.ram.get(0, PushSegment::Argument, 0);
        let index = self.ram.get(0, PushSegment::Argument, 1);
        VMString { address }.char_at(self, index).unwrap()
    }

    fn string_set_char_at(&mut self) -> Word {
        let address = self.ram.get(0, PushSegment::Argument, 0);
        let index = self.ram.get(0, PushSegment::Argument, 1);
        let new_value = self.ram.get(0, PushSegment::Argument, 2);
        let result = VMString { address }.set_char_at(self, index, new_value);
        if result.is_some() { 0 } else { panic!() }
    }

    fn string_append_char(&mut self) -> Word {
        let address = self.ram.get(0, PushSegment::Argument, 0);
        let new_char = self.ram.get(0, PushSegment::Argument, 1);
        let result = VMString { address }.append_char(self, new_char);

        if result.is_some() { address } else { panic!() }
    }

    fn string_erase_last_char(&mut self) -> Word {
        let address = self.ram.get(0, PushSegment::Argument, 0);
        let result = VMString { address }.erase_last_char(self);

        if result.is_some() { 0 } else { panic!() }
    }

    fn string_int_value(&mut self) -> Word {
        let address = self.ram.get(0, PushSegment::Argument, 0);
        VMString { address }.int_value(self).unwrap()
    }

    fn string_set_int(&mut self) -> Word {
        let address = self.ram.get(0, PushSegment::Argument, 0);
        let value = self.ram.get(0, PushSegment::Argument, 1);
        let result = VMString { address }.set_int(self, value);

        if result.is_some() { 0 } else { panic!() }
    }

    fn string_backspace(&mut self) -> Word {
        129
    }

    fn string_double_quote(&mut self) -> Word {
        34
    }

    fn string_new_line(&mut self) -> Word {
        128
    }

    fn output_move_cursor(&mut self) -> Word {
        let row = self.ram.get(0, PushSegment::Argument, 0);
        let col = self.ram.get(0, PushSegment::Argument, 1);

        if Output::move_cursor(self, row, col).is_some() {
            0
        } else {
            panic!()
        }
    }

    fn output_print_char(&mut self) -> Word {
        let c = self.ram.get(0, PushSegment::Argument, 0);
        Output::print_char(self, c);

        0
    }

    fn output_print_string(&mut self) -> Word {
        let address = self.ram.get(0, PushSegment::Argument, 0);
        let s = VMString { address };
        Output::print_string(self, s);

        0
    }

    fn output_print_int(&mut self) -> Word {
        let value = self.ram.get(0, PushSegment::Argument, 0);
        Output::print_int(self, value);

        0
    }

    fn output_println(&mut self) -> Word {
        self.os.output.println();

        0
    }

    fn output_backspace(&mut self) -> Word {
        Output::backspace(self);

        0
    }
}

#[derive(Clone)]
struct Output {
    row: Word,
    col: Word,
}

impl Output {
    fn draw_char(run_state: &mut RunState, c: Word) {
        let bitmap = character_bitmaps(c);
        const CHAR_WIDTH: Word = 8;
        const SUB_COLUMNS: Word = Word::BITS as Word / CHAR_WIDTH;
        let col = run_state.os.output.col;
        let row = run_state.os.output.row;

        for (i, mut row_bits) in bitmap.into_iter().enumerate() {
            let mut mask = 255;
            let offset = (col % SUB_COLUMNS) * CHAR_WIDTH;
            row_bits <<= offset;
            mask <<= offset;

            let address = RAM::SCREEN
                + RAM::SCREEN_ROW_LENGTH
                + (11 * row + i as Word) * RAM::SCREEN_ROW_LENGTH
                + col / SUB_COLUMNS;
            run_state.ram[address] &= !mask;
            run_state.ram[address] |= row_bits;
        }
    }

    fn move_cursor(run_state: &mut RunState, row: Word, col: Word) -> Option<()> {
        if !(0..=22).contains(&row) || !(0..63).contains(&col) {
            return None;
        }

        run_state.os.output.row = row;
        run_state.os.output.col = col;

        Self::draw_char(run_state, b' ' as Word);

        Some(())
    }

    fn println(&mut self) {
        self.row = (self.row + 1) % 23;
        self.col = 0;
    }

    fn backspace(run_state: &mut RunState) {
        let col = (run_state.os.output.col + 63) % 64;
        let row = if col == 63 {
            (run_state.os.output.row + 22) % 23
        } else {
            run_state.os.output.row
        };

        Output::move_cursor(run_state, row, col);
    }

    fn print_char(run_state: &mut RunState, c: Word) {
        if c == run_state.string_new_line() {
            return run_state.os.output.println();
        }

        if c == run_state.string_backspace() {
            return Self::backspace(run_state);
        }

        Self::draw_char(run_state, c);

        run_state.os.output.col = (run_state.os.output.col + 1) % 64;
        run_state.os.output.row = if run_state.os.output.col == 0 {
            (run_state.os.output.row + 1) % 23
        } else {
            run_state.os.output.row
        };
    }

    fn print_string(run_state: &mut RunState, s: VMString) {
        for i in 0..s.length(run_state) {
            let c = s.char_at(run_state, i).unwrap();
            Self::print_char(run_state, c);
        }
    }

    fn print_int(run_state: &mut RunState, value: Word) {
        let mut buffer = [0; 6];
        let mut index = 0;
        let mut remainder = (value as i32).abs();
        loop {
            buffer[index] = char::from_digit((remainder % 10) as u32, 10).unwrap() as u8 as Word;
            remainder /= 10;
            index += 1;
            if remainder == 0 {
                break;
            }
        }

        if value < 0 {
            buffer[index] = b'-' as Word;
            index += 1;
        }

        for i in 0..index {
            Self::print_char(run_state, buffer[index - i - 1]);
        }
    }
}

struct VMString {
    address: Word,
}

impl VMString {
    fn new(run_state: &mut RunState, capacity: Word) -> Option<Self> {
        let address = run_state.os.memory.alloc(2 + capacity)?;

        let instance = Self { address };

        *instance.length_mut(run_state) = 0;
        *instance.capacity_mut(run_state) = capacity;

        Some(instance)
    }

    fn char_at(&self, run_state: &RunState, index: Word) -> Option<Word> {
        if self.length(run_state) <= index {
            return None;
        }

        Some(run_state.ram[self.address + 2 + index])
    }

    fn set_char_at(&self, run_state: &mut RunState, index: Word, new_value: Word) -> Option<()> {
        if self.length(run_state) <= index {
            return None;
        }

        run_state.ram[self.address + 2 + index] = new_value;

        Some(())
    }

    fn append_char(&self, run_state: &mut RunState, new_char: Word) -> Option<()> {
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

    fn int_value(&self, run_state: &RunState) -> Option<Word> {
        if self.length(run_state) <= 0 {
            return None;
        }

        let mut start = self.address + 2;
        let is_negative = run_state.ram[start] == b'-' as Word;
        if is_negative {
            start += 1;
        }
        let value = run_state.ram.contents
            [start as usize..(self.address + 2 + self.length(run_state)) as usize]
            .iter()
            .try_fold(0, |acc, &i| {
                (i as u8 as char).to_digit(10).map(|d| acc * 10 + d as i64)
            });

        value.map(|v| if is_negative { -v } else { v } as Word)
    }

    fn set_int(&self, run_state: &mut RunState, value: Word) -> Option<()> {
        let mut buffer = [0; 11];
        let mut index = 0;
        let mut remainder = (value as i64).abs();
        loop {
            buffer[index] = char::from_digit((remainder % 10) as u32, 10).unwrap() as u8 as Word;
            remainder /= 10;
            index += 1;
            if remainder == 0 {
                break;
            }
        }

        if value < 0 {
            buffer[index] = b'-' as Word;
            index += 1;
        }

        if index > self.capacity(run_state) as usize {
            println!("{} {}", index, self.capacity(run_state) as usize);
            return None;
        }

        for i in 0..index {
            run_state.ram[self.address + 2 + i as Word] = buffer[index - i - 1];
        }
        *self.length_mut(run_state) = index as Word;

        Some(())
    }

    fn length(&self, run_state: &RunState) -> Word {
        run_state.ram[self.address]
    }

    fn length_mut<'a>(&self, run_state: &'a mut RunState) -> &'a mut Word {
        &mut run_state.ram[self.address]
    }

    fn capacity(&self, run_state: &RunState) -> Word {
        run_state.ram[self.address + 1]
    }

    fn capacity_mut<'a>(&self, run_state: &'a mut RunState) -> &'a mut Word {
        &mut run_state.ram[self.address + 1]
    }
}

#[derive(Clone)]
struct Memory {
    hole_starts: HashMap<Word, Word>,
    hole_ends: HashMap<Word, Word>,
    allocs: HashMap<Word, Word>,
}

#[derive(Clone)]
struct Screen {
    color: bool,
}

impl Memory {
    fn new(start_address: Word, size: Word) -> Self {
        Memory {
            hole_starts: HashMap::from_iter([(start_address, size)]),
            hole_ends: HashMap::from_iter([(start_address + size, size)]),
            allocs: HashMap::new(),
        }
    }

    fn alloc(&mut self, size: Word) -> Option<Word> {
        let (&hole_start, &hole_size) = self
            .hole_starts
            .iter()
            .find(|&(_, hole_size)| *hole_size >= size)?;

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

    fn dealloc(&mut self, address: Word) -> bool {
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
                breakpoints: vec![],
            };

            instance.ram[Register::ARG] = 100;

            instance
        }
    }

    #[test]
    fn test_string() {
        let mut run_state = RunState::test_instance();
        run_state.ram.set(0, PopSegment::Argument, 0, 11);
        let s = run_state.string_new();
        assert!(s.is_positive());

        run_state.ram.set(0, PopSegment::Argument, 0, s);
        assert_eq!(run_state.string_length(), 0);

        run_state
            .ram
            .set(0, PopSegment::Argument, 1, '5' as u8 as Word);
        assert_eq!(run_state.string_append_char(), s);
        assert_eq!(run_state.string_length(), 1);

        run_state.ram.set(0, PopSegment::Argument, 1, 0);
        assert_eq!(run_state.string_char_at(), '5' as u8 as Word);
        assert_eq!(run_state.string_int_value(), 5);

        run_state
            .ram
            .set(0, PopSegment::Argument, 2, '9' as u8 as Word);
        assert_eq!(run_state.string_set_char_at(), 0);
        assert_eq!(run_state.string_char_at(), '9' as u8 as Word);
        assert_eq!(run_state.string_int_value(), 9);

        run_state.ram.set(0, PopSegment::Argument, 1, Word::MAX);
        assert_eq!(run_state.string_set_int(), 0);
        assert_eq!(
            run_state.string_length(),
            Word::MAX.to_string().len() as Word
        );
        assert_eq!(run_state.string_int_value(), Word::MAX);

        run_state.ram.set(0, PopSegment::Argument, 1, Word::MIN);
        assert_eq!(run_state.string_set_int(), 0);
        assert_eq!(
            run_state.string_length(),
            Word::MIN.to_string().len() as Word
        );
        assert_eq!(run_state.string_int_value(), Word::MIN);
    }
}
