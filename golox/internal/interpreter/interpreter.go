package interpreter

import (
	"fmt"

	"github.com/michael-go/lox/golox/internal/ast"
	"github.com/michael-go/lox/golox/internal/globals"
	"github.com/michael-go/lox/golox/internal/token"
)

type Interpreter struct {
}

func New() Interpreter {
	return Interpreter{}
}

func (i *Interpreter) Interpret(expr ast.Expr) string {
	defer func() {
		if r := recover(); r != nil {
			if err, ok := r.(globals.RuntimeError); ok {
				globals.ReportRuntimeError(err)
			} else {
				panic(r)
			}
		}
	}()

	value := i.evaluate(expr)
	return stringify(value)
}

func stringify(obj any) string {
	if obj == nil {
		return "nil"
	}
	return fmt.Sprintf("%v", obj)
}

func (i *Interpreter) VisitLiteralExpr(expr ast.Literal) any {
	return expr.Value
}

func (i *Interpreter) VisitGroupingExpr(expr ast.Grouping) any {
	return i.evaluate(expr.Expression)
}

func (i *Interpreter) evaluate(expr ast.Expr) any {
	return expr.Accept(i)
}

func (i *Interpreter) VisitUnaryExpr(expr ast.Unary) any {
	right := i.evaluate(expr.Right)

	switch expr.Operator.Type {
	case token.MINUS:
		checkNumberOperand(expr.Operator, right)
		return -right.(float64)
	case token.BANG:
		return !isTruthy(right)
	}

	return nil
}

func (i *Interpreter) VisitBinaryExpr(expr ast.Binary) any {
	op := expr.Operator
	left := i.evaluate(expr.Left)
	right := i.evaluate(expr.Right)

	switch op.Type {
	case token.MINUS:
		checkNumberOperands(expr.Operator, left, right)
		return left.(float64) - right.(float64)
	case token.SLASH:
		checkNumberOperands(expr.Operator, left, right)
		return left.(float64) / right.(float64)
	case token.STAR:
		checkNumberOperands(expr.Operator, left, right)
		return left.(float64) * right.(float64)
	case token.PLUS:
		if leftIsNumber, ok := left.(float64); ok {
			if rightIsNumber, ok := right.(float64); ok {
				return leftIsNumber + rightIsNumber
			}
		}
		if leftIsString, ok := left.(string); ok {
			if rightIsString, ok := right.(string); ok {
				return leftIsString + rightIsString
			}
		}
		panic(globals.RuntimeError{Token: expr.Operator, Message: "Operands must be two numbers or two strings."})
	case token.GREATER:
		checkNumberOperands(expr.Operator, left, right)
		return left.(float64) > right.(float64)
	case token.GREATER_EQUAL:
		checkNumberOperands(expr.Operator, left, right)
		return left.(float64) >= right.(float64)
	case token.LESS:
		checkNumberOperands(expr.Operator, left, right)
		return left.(float64) < right.(float64)
	case token.LESS_EQUAL:
		checkNumberOperands(expr.Operator, left, right)
		return left.(float64) <= right.(float64)
	case token.BANG_EQUAL:
		return !isEqual(left, right)
	case token.EQUAL_EQUAL:
		return isEqual(left, right)
	}

	return nil
}

func isEqual(left any, right any) bool {
	if left == nil && right == nil {
		return true
	}
	if left == nil {
		return false
	}

	return left == right
}

func isTruthy(obj any) bool {
	if obj == nil {
		return false
	}
	if obj, ok := obj.(bool); ok {
		return obj
	}
	return true
}

func checkNumberOperand(operator token.Token, operand any) {
	if _, ok := operand.(float64); ok {
		return
	}
	panic(globals.RuntimeError{Token: operator, Message: "Operand must be a number."})
}

func checkNumberOperands(operator token.Token, left any, right any) {
	_, okLeft := left.(float64)
	_, okRight := right.(float64)
	if okLeft && okRight {
		return
	}
	panic(globals.RuntimeError{Token: operator, Message: "Operands must be numbers."})
}
