package interpreter

import "github.com/michael-go/lox/golox/internal/ast"

type LoxCallable interface {
	Arity() int
	Call(interpreter *Interpreter, arguments []any) any
}

type LoxFunction struct {
	declaration   *ast.Function
	closure       *Environment
	isInitializer bool
}

func NewLoxFunction(declaration *ast.Function, closure *Environment, isInitializer bool) *LoxFunction {
	return &LoxFunction{
		declaration:   declaration,
		closure:       closure,
		isInitializer: isInitializer,
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
				if f.isInitializer {
					ret = f.closure.GetAt(0, "this")
				} else {
					ret = err.Value
				}
			} else {
				panic(r)
			}
		}
	}()

	interpreter.executeBlock(f.declaration.Body, environment)
	if f.isInitializer {
		return f.closure.GetAt(0, "this")
	}
	ret = nil
	return
}

func (f LoxFunction) String() string {
	return "<fn " + f.declaration.Name.Lexeme + ">"
}

func (f LoxFunction) Bind(instance *LoxInstance) *LoxFunction {
	environment := NewEnvironment(f.closure)
	environment.Define("this", instance)
	return NewLoxFunction(f.declaration, environment, f.isInitializer)
}
