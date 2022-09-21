package interpreter

import (
	"fmt"

	"github.com/michael-go/lox/golox/internal/ast"
	"github.com/michael-go/lox/golox/internal/globals"
	"github.com/michael-go/lox/golox/internal/token"
)

type Interpreter struct {
	environment *Environment

	// declare like this to be able to mock it in tests
	Print func(str string)
}

func New() Interpreter {
	return Interpreter{
		environment: NewEnvironment(nil),
		Print: func(str string) {
			fmt.Print(str)
		},
	}
}

func (i *Interpreter) Interpret(statements []ast.Stmt) string {
	defer func() {
		if r := recover(); r != nil {
			if err, ok := r.(globals.RuntimeError); ok {
				globals.ReportRuntimeError(err)
			} else {
				panic(r)
			}
		}
	}()

	var value any
	for _, statement := range statements {
		value = i.execute(statement)
	}
	return stringify(value)
}

func (i *Interpreter) execute(stmt ast.Stmt) any {
	return stmt.Accept(i)
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

func (i *Interpreter) VisitExpressionStmt(stmt ast.Expression) any {
	i.evaluate(stmt.Expression)
	return nil
}

func (i *Interpreter) VisitPrintStmt(stmt ast.Print) any {
	value := i.evaluate(stmt.Expression)
	i.Print(fmt.Sprintln(stringify(value)))
	return nil
}

func (i *Interpreter) VisitVarStmt(stmt ast.Var) any {
	var value any
	if stmt.Initializer != nil {
		value = i.evaluate(stmt.Initializer)
	}

	i.environment.Define(stmt.Name.Lexeme, value)
	return nil
}

func (i *Interpreter) VisitVariableExpr(expr ast.Variable) any {
	return i.environment.Get(expr.Name)
}

func (i *Interpreter) VisitAssignExpr(expr ast.Assign) any {
	value := i.evaluate(expr.Value)
	i.environment.Assign(expr.Name, value)
	return value
}

func (i *Interpreter) VisitBlockStmt(stmt ast.Block) any {
	i.executeBlock(stmt.Statements, NewEnvironment(i.environment))
	return nil
}

func (i *Interpreter) executeBlock(statements []ast.Stmt, env *Environment) {
	previous := i.environment
	defer func() { i.environment = previous }()
	i.environment = env

	for _, statement := range statements {
		i.execute(statement)
	}
}

func (i *Interpreter) VisitIfStmt(stmt ast.If) any {
	if isTruthy(i.evaluate(stmt.Condition)) {
		i.execute(stmt.ThenBranch)
	} else if stmt.ElseBranch != nil {
		i.execute(stmt.ElseBranch)
	}
	return nil
}

func (i *Interpreter) VisitLogicalExpr(expr ast.Logical) any {
	left := i.evaluate(expr.Left)

	if expr.Operator.Type == token.OR {
		if isTruthy(left) {
			return left
		}
	} else {
		if !isTruthy(left) {
			return left
		}
	}

	return i.evaluate(expr.Right)
}

func (i *Interpreter) VisitWhileStmt(stmt ast.While) any {
	for isTruthy(i.evaluate(stmt.Condition)) {
		i.execute(stmt.Body)
	}
	return nil
}
