#define_import_path npbr::stack_machine

let PCODE_SIZE = 64;
let PCODE_UNIFORM_SIZE = 16; // PCODE_SIZE / 4
let STACK_SIZE = 16;

let OP_NOP = 0u;
let OP_LITERAL = 1u;
let OP_CONTEXT = 2u;
let OP_NEG = 3u;
let OP_ADD = 4u;
let OP_SUB = 5u;
let OP_MUL = 6u;
let OP_DIV = 7u;
let OP_MOD = 8u;
let OP_ABS = 9u;
let OP_MIN = 10u;
let OP_MAX = 11u;

struct StackMachineUniform {
    pcode: array<vec4<u32>, PCODE_UNIFORM_SIZE>,
    push_buf: array<vec4<f32>, PCODE_UNIFORM_SIZE>,
    context_idx_buf: array<vec4<u32>, PCODE_UNIFORM_SIZE>,
}

struct StackMachine {
    pcode: array<u32, PCODE_SIZE>,

    push_buf: array<f32, PCODE_SIZE>,
    push_buf_pointer: i32,

    context_buf: array<f32, PCODE_SIZE>,
    context_idx_buf: array<u32, PCODE_SIZE>,
    context_idx_buf_ptr: i32,

    stack: array<f32, STACK_SIZE>,
    stack_pointer: i32,
}

fn stack_machine_new() -> StackMachine {
    var stack_machine = StackMachine(
        array<u32, PCODE_SIZE>(),
        array<f32, PCODE_SIZE>(),
        0,
        array<f32, PCODE_SIZE>(),
        array<u32, PCODE_SIZE>(),
        0,
        array<f32, STACK_SIZE>(),
        -1
    );

    return stack_machine;
}

fn stack_machine_decode(
    stack_machine: ptr<function, StackMachine>
) {
    for(var i = 0u; i < u32(PCODE_SIZE); i++) {
        (*stack_machine).pcode[i] = stack_machine_uniform.pcode[i / 4u][i % 4u];
        (*stack_machine).push_buf[i] = stack_machine_uniform.push_buf[i / 4u][i % 4u];
        (*stack_machine).context_idx_buf[i] = stack_machine_uniform.context_idx_buf[i / 4u][i % 4u];
    }
}

fn stack_machine_push(stack_machine: ptr<function, StackMachine>, t: f32) {
    (*stack_machine).stack_pointer += 1;
    (*stack_machine).stack[(*stack_machine).stack_pointer] = t;
}

fn stack_machine_literal(
    stack_machine: ptr<function, StackMachine>,
) {
    let t = (*stack_machine).push_buf[(*stack_machine).push_buf_pointer];
    (*stack_machine).push_buf_pointer += 1;
    stack_machine_push(stack_machine, t);
}

fn stack_machine_context(
    stack_machine: ptr<function, StackMachine>,
) {
    let idx_idx = (*stack_machine).context_idx_buf_ptr;
    let idx = (*stack_machine).context_idx_buf[idx_idx];
    let t = (*stack_machine).context_buf[idx];
    (*stack_machine).context_idx_buf_ptr += 1;
    stack_machine_push(stack_machine, t);
}

fn stack_machine_pop(
    stack_machine: ptr<function, StackMachine>,
) -> f32 {
    let t = (*stack_machine).stack[(*stack_machine).stack_pointer];
    (*stack_machine).stack[(*stack_machine).stack_pointer] = 0.0;
    (*stack_machine).stack_pointer -= 1;
    return t;
}

fn stack_machine_peek(
    stack_machine: ptr<function, StackMachine>,
) -> f32 {
    return (*stack_machine).stack[(*stack_machine).stack_pointer];
}

fn stack_machine_poke(
    stack_machine: ptr<function, StackMachine>,
    t: f32,
) {
    (*stack_machine).stack[(*stack_machine).stack_pointer] = t;
}

fn stack_machine_pop_peek(
    stack_machine: ptr<function, StackMachine>,
) -> array<f32, 2> {
    let rhs = stack_machine_pop(stack_machine);
    let lhs = stack_machine_peek(stack_machine);
    return array<f32, 2>(lhs, rhs);
}

fn stack_machine_neg(stack_machine: ptr<function, StackMachine>) {
    stack_machine_poke(
        stack_machine,
        -stack_machine_peek(stack_machine)
    );
}

fn stack_machine_add(stack_machine: ptr<function, StackMachine>) {
    let two = stack_machine_pop_peek(stack_machine);
    stack_machine_poke(stack_machine, two[0] + two[1]);
}

fn stack_machine_sub(stack_machine: ptr<function, StackMachine>) {
    let two = stack_machine_pop_peek(stack_machine);
    stack_machine_poke(stack_machine, two[0] - two[1]);
}

fn stack_machine_mul(stack_machine: ptr<function, StackMachine>) {
    let two = stack_machine_pop_peek(stack_machine);
    stack_machine_poke(stack_machine, two[0] * two[1]);
}

fn stack_machine_div(stack_machine: ptr<function, StackMachine>) {
    let two = stack_machine_pop_peek(stack_machine);
    stack_machine_poke(stack_machine, two[0] / two[1]);
}

fn stack_machine_mod(stack_machine: ptr<function, StackMachine>) {
    let two = stack_machine_pop_peek(stack_machine);
    stack_machine_poke(stack_machine, two[0] % two[1]);
}

fn stack_machine_abs(stack_machine: ptr<function, StackMachine>) {
        stack_machine_poke(
            stack_machine,
            abs(
                stack_machine_peek(stack_machine)
            )
        );
}

fn stack_machine_min(stack_machine: ptr<function, StackMachine>) {
    let two = stack_machine_pop_peek(stack_machine);
    stack_machine_poke(stack_machine, min(two[0], two[1]));
}

fn stack_machine_max(stack_machine: ptr<function, StackMachine>) {
    let two = stack_machine_pop_peek(stack_machine);
    stack_machine_poke(stack_machine, max(two[0], two[1]));
}

fn stack_machine_tick(
    stack_machine: ptr<function, StackMachine>,
    op: u32
) -> bool {
    if op == OP_NOP {
        return false;
    }
    else if op == OP_LITERAL {
        stack_machine_literal(stack_machine);
    }
    else if op == OP_CONTEXT {
        stack_machine_context(stack_machine);
    }
    else if op == OP_NEG {
        stack_machine_neg(stack_machine);
    }
    else if op == OP_ADD {
        stack_machine_add(stack_machine);
    }
    else if op == OP_SUB {
        stack_machine_sub(stack_machine);
    }
    else if op == OP_MUL {
        stack_machine_mul(stack_machine);
    }
    else if op == OP_DIV {
        stack_machine_div(stack_machine);
    }
    else if op == OP_MOD {
        stack_machine_mod(stack_machine);
    }
    else if op == OP_ABS {
        stack_machine_abs(stack_machine);
    }
    else if op == OP_MIN {
        stack_machine_min(stack_machine);
    }
    else if op == OP_MAX {
        stack_machine_max(stack_machine);
    }

    return true;
}

fn stack_machine_run(
    stack_machine: ptr<function, StackMachine>,
) {
    for(var i = 0u; i < u32(PCODE_SIZE); i++) {
        let op = (*stack_machine).pcode[i];
        if !stack_machine_tick(stack_machine, op) {
            break;
        }
    }
}

