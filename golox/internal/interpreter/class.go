package interpreter

import (
	"github.com/michael-go/lox/golox/internal/globals"
	"github.com/michael-go/lox/golox/internal/token"
)

type ILoxClass interface {
	FindMethod(name string) *LoxFunction
}

type LoxClass struct {
	name       string
	superclass ILoxClass
	methods    map[string]*LoxFunction
}

type LoxInstance struct {
	class  *LoxClass
	fields map[string]any
}

func NewLoxClass(name string, superclass ILoxClass, methods map[string]*LoxFunction) *LoxClass {
	return &LoxClass{
		name:       name,
		superclass: superclass,
		methods:    methods,
	}
}

func (c *LoxClass) String() string {
	return c.name
}

func (c *LoxClass) Arity() int {
	initializer := c.FindMethod("init")
	if initializer == nil {
		return 0
	}
	return initializer.Arity()
}

func (c *LoxClass) Call(interpreter *Interpreter, arguments []any) any {
	instance := NewLoxInstance(c)
	if initializer := c.methods["init"]; initializer != nil {
		initializer.Bind(instance).Call(interpreter, arguments)
	}
	return instance
}

func (i *LoxClass) FindMethod(name string) *LoxFunction {
	if method, ok := i.methods[name]; ok {
		return method
	}

	// TODO: this is quite hacky, kinda makes the interface not used as intended
	//  but was a way to detect interface wrapping a nil value
	//  for related discussion see: https://stackoverflow.com/questions/13476349/check-for-nil-and-nil-interface-in-go
	if super, ok := i.superclass.(*LoxClass); ok && super != nil {
		return super.FindMethod(name)
	}

	return nil
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

	method := i.class.FindMethod(name.Lexeme)
	if method != nil {
		// TODO: bind to what?
		method := method.Bind(i)
		return method
	}

	panic(globals.RuntimeError{Token: name, Message: "Undefined property '" + name.Lexeme + "'."})
}

func (i *LoxInstance) Set(name token.Token, value any) {
	i.fields[name.Lexeme] = value
}
