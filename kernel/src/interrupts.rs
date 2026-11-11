use crate::diagnostics::{self, Diagnostic};
use crate::gdt;
use crate::logger::{Level, Subsystem};
use pic8259::ChainedPics;
use spin;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
    /// COM1 is wired to IRQ 4.
    Serial = PIC_1_OFFSET + 4,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }
}

lazy_static::lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.divide_error.set_handler_fn(divide_error_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }

        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);

        // Hardware interrupts (IRQs). The IDT can be indexed directly by vector number.
        idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::Serial.as_u8()].set_handler_fn(serial_interrupt_handler);

        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    crate::time::tick();

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let scancode = crate::input::keyboard::read_scancode();
    crate::input::keyboard::handle_scancode(scancode);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

extern "x86-interrupt" fn serial_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // One interrupt can stand for several bytes in the UART's FIFO.
    while let Some(byte) = crate::serial::try_read_byte() {
        crate::input::serial::handle_byte(byte);
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Serial.as_u8());
    }
}

/// Common context for exception diagnostics: where the CPU was.
fn exception_diagnostic(
    severity: Level,
    code: &'static str,
    message: &'static str,
    stack_frame: &InterruptStackFrame,
) -> Diagnostic {
    Diagnostic::new(
        severity,
        Subsystem::Interrupt,
        code,
        format_args!("{}", message),
    )
    .with(
        "rip",
        format_args!("{:#x}", stack_frame.instruction_pointer.as_u64()),
    )
    .with(
        "rsp",
        format_args!("{:#x}", stack_frame.stack_pointer.as_u64()),
    )
}

/// #BP is the one exception that is not an error: execution continues after
/// the `int3` instruction.
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    diagnostics::report(exception_diagnostic(
        Level::Info,
        "EXC-003",
        "Breakpoint exception",
        &stack_frame,
    ));
}

extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    crate::panic::raise(exception_diagnostic(
        Level::Panic,
        "EXC-000",
        "Divide error during kernel execution",
        &stack_frame,
    ));
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    crate::panic::raise(exception_diagnostic(
        Level::Panic,
        "EXC-006",
        "Invalid opcode during kernel execution",
        &stack_frame,
    ));
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    let diagnostic = exception_diagnostic(
        Level::Panic,
        "EXC-014",
        "Page fault during kernel execution",
        &stack_frame,
    );
    // CR2 holds the address whose access faulted.
    let diagnostic = match Cr2::read() {
        Ok(address) => diagnostic.with("address", format_args!("{:#x}", address.as_u64())),
        Err(_) => diagnostic.with("address", "non-canonical"),
    };
    let access = if error_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH) {
        "execute"
    } else if error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE) {
        "write"
    } else {
        "read"
    };
    let cause = if error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
        "protection violation"
    } else {
        "page not present"
    };
    crate::panic::raise(
        diagnostic
            .with("access", access)
            .with("cause", cause)
            .with("error_code", format_args!("{:#x}", error_code.bits())),
    );
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    crate::panic::raise(
        exception_diagnostic(
            Level::Panic,
            "EXC-013",
            "General protection fault during kernel execution",
            &stack_frame,
        )
        // Non-zero: the segment selector the fault relates to.
        .with("error_code", format_args!("{:#x}", error_code)),
    );
}

/// Runs on its own stack (IST), so it still works when the kernel stack has
/// overflowed, which is the usual way to get here.
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    crate::panic::raise(
        exception_diagnostic(
            Level::Panic,
            "EXC-008",
            "Double fault during kernel execution",
            &stack_frame,
        )
        .with("hint", "often caused by a kernel stack overflow"),
    );
}

#[test_case]
fn breakpoint_exception_returns_and_is_recorded() {
    let reported = diagnostics::total_reported();
    x86_64::instructions::interrupts::int3();
    // Reaching this line proves the handler returned.
    assert_eq!(diagnostics::total_reported(), reported + 1);

    let record = diagnostics::last().expect("breakpoint must leave a diagnostic");
    assert_eq!(record.code, "EXC-003");
    assert_eq!(record.severity, Level::Info);
    assert!(record.context_value("rip").is_some());
}
