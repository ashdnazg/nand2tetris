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
            screen: Screen { color: false },
        }
    }
}

type Func = fn(&mut RunState) -> i16;

impl RunState {
    pub fn call_os(&mut self, function_name: &str) -> bool {
        let function = match function_name {
            "Math.multiply" => Self::math_multiply,
            "Math.divide" => Self::math_divide,
            "Math.min" => Self::math_min,
            "Math.max" => Self::math_max,
            "Math.sqrt" => Self::math_sqrt,
            "Array.new" => Self::memory_alloc,
            "Array.dispose" => Self::memory_dealloc,
            "Keyboard.keyPressed" => Self::keyboard_key_pressed,
            "Screen.setColor" => Self::screen_set_color,
            "Screen.drawPixel" => Self::screen_draw_pixel,
            "Memory.peek" => Self::memory_peek,
            "Memory.poke" => Self::memory_poke,
            "Memory.alloc" => Self::memory_alloc,
            "Memory.deAlloc" => Self::memory_dealloc,
            _ => return false,
        };

        self.call(function);

        true
    }

    fn call(&mut self, f: Func) {
        let return_value = f(self);
        self.ram.push(return_value);
    }

    fn math_multiply(&mut self) -> i16 {
        let x = self.ram.get(0, PushSegment::Argument, 0);
        let y = self.ram.get(0, PushSegment::Argument, 1);

        x * y
    }

    fn math_divide(&mut self) -> i16 {
        let x = self.ram.get(0, PushSegment::Argument, 0);
        let y = self.ram.get(0, PushSegment::Argument, 1);

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
            return -1;
        }

        (x as f64).sqrt().floor() as i16
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
        self.os.memory.alloc(size).unwrap_or(-1)
    }

    fn memory_dealloc(&mut self) -> i16 {
        let object = self.ram.get(0, PushSegment::Argument, 0);
        if self.os.memory.dealloc(object) {
            0
        } else {
            -1
        }
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
