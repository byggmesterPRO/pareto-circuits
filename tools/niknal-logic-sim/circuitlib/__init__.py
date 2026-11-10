from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

class Direction(Enum):
    PosX = 1
    PosY = 2
    PosZ = 3
    NegX = 4
    NegY = 5
    NegZ = 6

    def asIVec3(self) -> IVec3:
        match self:
            case Direction.PosX: return IVec3(1, 0, 0)
            case Direction.PosY: return IVec3(0, 1, 0)
            case Direction.PosZ: return IVec3(0, 0, 1)
            case Direction.NegX: return IVec3(-1, 0, 0)
            case Direction.NegY: return IVec3(0, -1, 0)
            case Direction.NegZ: return IVec3(0, 0, -1)

@dataclass(frozen=True)
class IVec3:
    x: int
    y: int
    z: int

    def __add__(self, other: IVec3 | Direction | tuple[int, int, int]):
        if isinstance(other, IVec3):
            return IVec3(
                self.x + other.x,
                self.y + other.y,
                self.z + other.z
            )
        elif isinstance(other, Direction):
            return self + other.asIVec3()
        elif isinstance(other, tuple):
            return self + IVec3(other[0], other[1], other[2])
        return NotImplemented

class CircuitElement:
    def __init__(self, position: IVec3):
        self.inputs: set[CircuitElement] = set()
        self.outputs: set[CircuitElement] = set()
        self.position = position

    def connect_to(self, cel: CircuitElement) -> None:
        self.outputs.add(cel)
        cel.inputs.add(self)

    def occupied_positions(self) -> set[IVec3]:
        return {self.position}

class Timer(CircuitElement):
    def __init__(self, position: IVec3, direction: Direction, delay: int):
        super().__init__(position)
        self.direction = direction

        assert 0 <= delay <= 2400
        self.delay = delay

    def occupied_positions(self) -> set[IVec3]:
        return {self.position, self.position + self.direction}

class GateType(Enum):
    AND = 1
    OR = 2
    XOR = 3
    NAND = 4
    NOR = 5
    XNOR = 6

class Gate(CircuitElement):
    def __init__(self, position: IVec3, gate_type: GateType):
        super().__init__(position)

        assert isinstance(gate_type, GateType)
        self.gate_type = gate_type

@dataclass
class ValidationConfig:
    max_inputs: int
    max_outputs: int
    max_outputs_per_circuit_input: int
    allow_selfwiring: bool
    allow_twoloop: bool

class Circuit:
    def __init__(self):
        self.cels: set[CircuitElement] = set()

        self.inputs: dict[str, set[CircuitElement]] = {}
        self.outputs: dict[str, CircuitElement] = {}

    def add_input(self, name: str) -> None:
        assert name not in self.inputs
        self.inputs[name] = set()

    def connect_input(self, name: str, cel: CircuitElement) -> None:
        assert name in self.inputs
        assert cel in self.cels
        self.inputs[name].add(cel)

    def add_output(self, name: str, cel: CircuitElement) -> None:
        assert name not in self.outputs
        assert cel in self.cels
        self.outputs[name] = cel

    def add_gate(self, position: IVec3, gate_type: GateType) -> Gate:
        g = Gate(position, gate_type)
        self.cels.add(g)
        return g

    def add_timer(self, position: IVec3, direction: Direction, delay: int) -> Timer:
        t = Timer(position, direction, delay)
        self.cels.add(t)
        return t

    def num_connections(self) -> int:
        element_connections = sum(len(cel.outputs) for cel in self.cels)
        input_connections = sum(len(targets) for targets in self.inputs.values())
        return element_connections + input_connections

    def validate(self, validation_config: ValidationConfig):
        for input_name, target_cels in self.inputs.items():
            if len(target_cels) > validation_config.max_outputs_per_circuit_input:
                return False, f"Input {input_name} has too many targets: {len(target_cels)} (max is {validation_config.max_outputs_per_circuit_input})"

        positions_taken = set()
        for cel in self.cels:
            occupied = cel.occupied_positions()
            if occupied & positions_taken:
                return False, f"Overlapping positions: {occupied & positions_taken}"
            positions_taken |= occupied

            num_inputs = len(cel.inputs)
            num_outputs = len(cel.outputs)

            for _, target_cels in self.inputs.items():
                if cel in target_cels:
                    num_inputs += 1
            for _, target_cel in self.outputs.items():
                if cel is target_cel:
                    num_outputs += 1

            if isinstance(cel, Timer) and num_inputs > 1:
                return False, f"Timer has {num_inputs}, but at most one is allowed"
            if num_inputs > validation_config.max_inputs:
                return False, f"Cel exceeded maximum input limit of {validation_config.max_inputs}. (actual: {num_inputs})"
            if num_outputs > validation_config.max_outputs:
                return False, f"Cel exceeded maximum output limit of {validation_config.max_outputs}. (actual: {num_outputs})"
            if (not validation_config.allow_selfwiring) and (cel in cel.outputs):
                return False, f"Cel is wired to itself"
            if not validation_config.allow_twoloop:
                for output in cel.outputs:
                    if output is not cel and cel in output.outputs:
                        return False, "Cel is cross-wired with another Cel (Cel1 -> Cel2 -> Cel1)"

        if positions_taken:
            remaining = positions_taken.copy()
            pending = {remaining.pop()}

            while pending:
                position = pending.pop()
                for direction in list(Direction):
                    neighbor = position + direction
                    if neighbor in remaining:
                        remaining.remove(neighbor)
                        pending.add(neighbor)

            if remaining:
                return False, "Some parts of the circuit are disconnected"

        return True, None

class Simulation:
    """A synchronous simulator that evaluates only affected elements."""

    def __init__(self, circuit: Circuit):
        self.circuit = circuit
        self.cels = list(circuit.cels)
        self.index = {cel: i for i, cel in enumerate(self.cels)}
        element_count = len(self.cels)

        self.values = [False] * element_count
        self.total_inputs = [0] * element_count
        self.on_inputs = [0] * element_count
        downstream: list[list[int]] = [[] for _ in range(element_count)]

        self.gate_types: list[GateType | None] = [None] * element_count
        self.timer_delays: list[int | None] = [None] * element_count
        self.timer_inputs = [False] * element_count

        for destination, cel in enumerate(self.cels):
            if isinstance(cel, Gate):
                if not isinstance(cel.gate_type, GateType):
                    raise ValueError(f"Unsupported gate type: {cel.gate_type!r}")
                self.gate_types[destination] = cel.gate_type
            elif isinstance(cel, Timer):
                self.timer_delays[destination] = cel.delay
            else:
                raise TypeError(f"Unsupported circuit element: {type(cel).__name__}")

            for source in cel.inputs:
                try:
                    source_index = self.index[source]
                except KeyError as error:
                    raise ValueError(
                        "Circuit input source is not in circuit.cels"
                    ) from error
                downstream[source_index].append(destination)
                self.total_inputs[destination] += 1

        self.downstream = [tuple(destinations) for destinations in downstream]

        self.input_names = list(circuit.inputs)
        self.input_index = {name: i for i, name in enumerate(self.input_names)}
        self.input_values = [False] * len(self.input_names)
        input_destinations: list[tuple[int, ...]] = []
        for name in self.input_names:
            destinations: list[int] = []
            for cel in circuit.inputs[name]:
                try:
                    destination = self.index[cel]
                except KeyError as error:
                    raise ValueError(
                        f"Destination of circuit input {name!r} is not in "
                        "circuit.cels"
                    ) from error
                destinations.append(destination)
                self.total_inputs[destination] += 1
            input_destinations.append(tuple(destinations))
        self.input_destinations = input_destinations

        for i, cel in enumerate(self.cels):
            if isinstance(cel, Timer) and self.total_inputs[i] > 1:
                raise ValueError(
                    f"Timer at {cel.position!r} has {self.total_inputs[i]} "
                    "inputs; at most one is supported"
                )

        outputs: dict[str, int] = {}
        for name, cel in circuit.outputs.items():
            try:
                outputs[name] = self.index[cel]
            except KeyError as error:
                raise ValueError(
                    f"Circuit output {name!r} refers to an element outside "
                    "circuit.cels"
                ) from error
        self.outputs = outputs

        self._pending = {
            i
            for i, gate_type in enumerate(self.gate_types)
            if gate_type is not None and self.total_inputs[i] > 0
        }
        self._tick_number = 0
        self._scheduled_timer_outputs: dict[int, list[tuple[int, bool]]] = {}

    def set_input(self, name: str, state: bool) -> None:
        input_index = self.input_index[name]
        state = bool(state)
        old_state = self.input_values[input_index]
        if state == old_state:
            return

        self.input_values[input_index] = state
        delta = 1 if state else -1
        for destination in self.input_destinations[input_index]:
            self.on_inputs[destination] += delta
            self._pending.add(destination)

    def read_output(self, name: str) -> bool:
        return self.values[self.outputs[name]]

    def tick(self) -> None:
        self._tick_number += 1
        tick_number = self._tick_number
        current = self._pending
        self._pending = set()

        planned: dict[int, bool] = {
            index: state
            for index, state in self._scheduled_timer_outputs.pop(tick_number, ())
        }

        for index in current:
            gate_type = self.gate_types[index]
            if gate_type is not None:
                planned[index] = self._evaluate_gate(
                    gate_type,
                    self.total_inputs[index],
                    self.on_inputs[index],
                )
                continue

            delay = self.timer_delays[index]
            if delay is None or self.total_inputs[index] == 0:
                continue

            sampled = self.on_inputs[index] == 1
            if sampled == self.timer_inputs[index]:
                continue
            self.timer_inputs[index] = sampled

            due_tick = tick_number + delay
            if delay == 0:
                planned[index] = sampled
            else:
                self._scheduled_timer_outputs.setdefault(due_tick, []).append(
                    (index, sampled)
                )

        for index, state in planned.items():
            old_state = self.values[index]
            if state == old_state:
                continue

            self.values[index] = state
            delta = 1 if state else -1
            for destination in self.downstream[index]:
                self.on_inputs[destination] += delta
                self._pending.add(destination)

    @staticmethod
    def _evaluate_gate(gate_type: GateType, total: int, on: int) -> bool:
        if total == 0:
            return False
        if gate_type is GateType.AND:
            return on == total
        if gate_type is GateType.OR:
            return on > 0
        if gate_type is GateType.XOR:
            return bool(on & 1)
        if gate_type is GateType.NAND:
            return on != total
        if gate_type is GateType.NOR:
            return on == 0
        if gate_type is GateType.XNOR:
            return not bool(on & 1)
        raise ValueError(f"Unsupported gate type: {gate_type!r}")

@dataclass(frozen=True)
class SetInput:
    name: str
    value: bool

@dataclass(frozen=True)
class CheckOutput:
    name: str
    expected: bool

@dataclass(frozen=True)
class Tick:
    ticks: int

TestAction = SetInput | CheckOutput | Tick

@dataclass
class TestCase:
    actions: list[TestAction]

def run_test(circuit: Circuit, testcase: TestCase) -> tuple[int, int]:
    sim = Simulation(circuit)
    correct = 0
    total = 0

    for action in testcase.actions:
        match action:
            case SetInput(name, value):
                sim.set_input(name, value)

            case CheckOutput(name, expected):
                total += 1
                if sim.read_output(name) == expected:
                    correct += 1

            case Tick(ticks):
                for _ in range(ticks):
                    sim.tick()

    return correct, total
