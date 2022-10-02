package interpreter

import (
	"github.com/michael-go/lox/golox/internal/globals"
	"github.com/michael-go/lox/golox/internal/token"
)

type LoxClass struct {
	name    string
	methods map[string]*LoxFunction
}

type LoxInstance struct {
	class  *LoxClass
	fields map[string]any
}

func NewLoxClass(name string, methods map[string]*LoxFunction) *LoxClass {
	return &LoxClass{
		name:    name,
		methods: methods,
	}
}

func (c *LoxClass) String() string {
	return c.name
}

func (c *LoxClass) Arity() int {
	initializer := c.methods["init"]
	if initializer == nil {
		return 0
	}
	return initializer.Arity()
}

func (c *LoxClass) Call(interpreter *Interpreter, arguments []any) any {
	instance := NewLoxInstance(c)
	if initializer := c.methods["init"]; initializer != nil {
		initializer.Bind(instance, true).Call(interpreter, arguments)
	}
	return instance
}

func NewLoxInstance(class *LoxClass) *LoxInstance {
	return &LoxInstance{
		class:  class,
		fields: make(map[string]any),
	}
}

func (i *LoxInstance) String() string {
	return i.class.name + " instance"
}

func (i *LoxInstance) Get(name token.Token) any {
	if value, ok := i.fields[name.Lexeme]; ok {
		return value
	}

	method := i.class.methods[name.Lexeme]
	if method != nil {
		method := method.Bind(i, method.isInitializer)
		return method
	}

	panic(globals.RuntimeError{Token: name, Message: "Undefined property '" + name.Lexeme + "'."})
}

func (i *LoxInstance) Set(name token.Token, value any) {
	i.fields[name.Lexeme] = value
}
