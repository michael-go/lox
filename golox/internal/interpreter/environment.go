package interpreter

import (
	"github.com/michael-go/lox/golox/internal/globals"
	"github.com/michael-go/lox/golox/internal/token"
)

type Environment struct {
	values    map[string]any
	enclosing *Environment
}

func NewEnvironment(enclosing *Environment) *Environment {
	return &Environment{
		values:    make(map[string]any),
		enclosing: enclosing,
	}
}

func (e *Environment) Define(name string, value any) {
	e.values[name] = value
}

func (e *Environment) Get(name token.Token) any {
	if value, ok := e.values[name.Lexeme]; ok {
		return value
	}

	if e.enclosing != nil {
		return e.enclosing.Get(name)
	}

	panic(globals.RuntimeError{
		Token:   name,
		Message: "Undefined variable '" + name.Lexeme + "'.",
	})
}

func (e *Environment) Assign(name token.Token, value any) {
	if _, ok := e.values[name.Lexeme]; ok {
		e.values[name.Lexeme] = value
		return
	}

	if e.enclosing != nil {
		e.enclosing.Assign(name, value)
		return
	}

	panic(globals.RuntimeError{
		Token:   name,
		Message: "Undefined variable '" + name.Lexeme + "'.",
	})
}
