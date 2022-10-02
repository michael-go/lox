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

func (e *Environment) GetAt(distance int, name string) any {
	return e.ancestor(distance).values[name]
}

func (e *Environment) ancestor(distance int) *Environment {
	env := e
	for i := 0; i < distance; i++ {
		env = env.enclosing
	}
	return env
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

func (e *Environment) AssignAt(distance int, name token.Token, value any) {
	e.ancestor(distance).values[name.Lexeme] = value
}
