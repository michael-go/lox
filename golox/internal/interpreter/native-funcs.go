package interpreter

import "time"

type ClockFunc struct{}

func (ClockFunc) Arity() int {
	return 0
}

func (ClockFunc) Call(interpreter *Interpreter, arguments []any) any {
	return float64(time.Now().UnixMilli()) / 1000
}

func (ClockFunc) String() string {
	return "<native fn>"
}
