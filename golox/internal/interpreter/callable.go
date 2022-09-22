package interpreter

import "github.com/michael-go/lox/golox/internal/ast"

type LoxCallable interface {
	Arity() int
	Call(interpreter *Interpreter, arguments []any) any
}

type LoxFunction struct {
	declaration *ast.Function
	closure     *Environment
}

func NewLoxFunction(declaration *ast.Function, closure *Environment) *LoxFunction {
	return &LoxFunction{
		declaration: declaration,
		closure:     closure,
	}
}

func (f LoxFunction) Arity() int {
	return len(f.declaration.Params)
}

func (f LoxFunction) Call(interpreter *Interpreter, arguments []any) (ret any) {
	environment := NewEnvironment(f.closure)

	for i, param := range f.declaration.Params {
		environment.Define(param.Lexeme, arguments[i])
	}

	defer func() {
		if r := recover(); r != nil {
			if err, ok := r.(Return); ok {
				ret = err.Value
			} else {
				panic(r)
			}
		}
	}()

	interpreter.executeBlock(f.declaration.Body, environment)
	ret = nil
	return
}

func (f LoxFunction) String() string {
	return "<fn " + f.declaration.Name.Lexeme + ">"
}
