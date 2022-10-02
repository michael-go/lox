package resolver

import (
	"github.com/michael-go/lox/golox/internal/ast"
	"github.com/michael-go/lox/golox/internal/globals"
	"github.com/michael-go/lox/golox/internal/interpreter"
	"github.com/michael-go/lox/golox/internal/token"
)

type FunctionType int

const (
	NOT_FUNC FunctionType = iota
	FUNCTION
	INITIALIZER
	METHOD
)

type ClassType int

const (
	NOT_CLASS ClassType = iota
	CLASS
)

type Resolver struct {
	interp              *interpreter.Interpreter
	scopes              []map[string]bool
	currentFunctionType FunctionType
	currentClassType    ClassType
}

func New(interp *interpreter.Interpreter) Resolver {
	return Resolver{
		interp: interp,
	}
}

func (r *Resolver) Resolve(statements []ast.Stmt) any {
	for _, statement := range statements {
		r.resolveStmt(statement)
	}
	return nil
}

func (r *Resolver) VisitBlockStmt(stmt *ast.Block) any {
	r.beginScope()
	r.Resolve(stmt.Statements)
	r.endScope()
	return nil
}

func (r *Resolver) resolveStmt(stmt ast.Stmt) {
	stmt.Accept(r)
}

func (r *Resolver) resolveExpr(expr ast.Expr) {
	expr.Accept(r)
}

func (r *Resolver) VisitExpressionStmt(stmt *ast.Expression) any {
	r.resolveExpr(stmt.Expression)
	return nil
}

func (r *Resolver) beginScope() {
	r.scopes = append(r.scopes, make(map[string]bool, 0))
}

func (r *Resolver) endScope() {
	r.scopes = r.scopes[:len(r.scopes)-1]
}

func (r *Resolver) VisitVarStmt(stmt *ast.Var) any {
	r.declare(stmt.Name)
	if stmt.Initializer != nil {
		r.resolveExpr(stmt.Initializer)
	}
	r.define(stmt.Name)
	return nil
}

func (r *Resolver) declare(name token.Token) {
	if len(r.scopes) == 0 {
		return
	}
	scope := r.scopes[len(r.scopes)-1]
	if _, ok := scope[name.Lexeme]; ok {
		globals.ReportError(name.Line, "", "Already a variable with this name in this scope.")
	}
	scope[name.Lexeme] = false
}

func (r *Resolver) define(name token.Token) {
	if len(r.scopes) == 0 {
		return
	}
	scope := r.scopes[len(r.scopes)-1]
	scope[name.Lexeme] = true
}

func (r *Resolver) VisitVariableExpr(expr *ast.Variable) any {
	if len(r.scopes) != 0 {
		scope := r.scopes[len(r.scopes)-1]

		if _, ok := scope[expr.Name.Lexeme]; ok && !scope[expr.Name.Lexeme] {
			globals.ReportError(expr.Name.Line, "", "Can't read local variable in its own initializer.")
		}
	}

	r.resolveLocal(expr, expr.Name)
	return nil
}

func (r *Resolver) resolveLocal(expr ast.Expr, name token.Token) {
	for i := len(r.scopes) - 1; i >= 0; i-- {
		if _, ok := r.scopes[i][name.Lexeme]; ok {
			r.interp.Resolve(expr, len(r.scopes)-1-i)
			return
		}
	}
}

func (r *Resolver) VisitAssignExpr(expr *ast.Assign) any {
	r.resolveExpr(expr.Value)
	r.resolveLocal(expr, expr.Name)
	return nil
}

func (r *Resolver) VisitFunctionStmt(stmt *ast.Function) any {
	r.declare(stmt.Name)
	r.define(stmt.Name)

	r.resolveFunction(stmt, FUNCTION)
	return nil
}

func (r *Resolver) resolveFunction(stmt *ast.Function, funcType FunctionType) any {
	encosingFunction := r.currentFunctionType
	r.currentFunctionType = funcType

	r.beginScope()
	for _, param := range stmt.Params {
		r.declare(param)
		r.define(param)
	}
	r.Resolve(stmt.Body)
	r.endScope()

	r.currentFunctionType = encosingFunction
	return nil
}

func (r *Resolver) VisitIfStmt(stmt *ast.If) any {
	r.resolveExpr(stmt.Condition)
	r.resolveStmt(stmt.ThenBranch)
	if stmt.ElseBranch != nil {
		r.resolveStmt(stmt.ElseBranch)
	}
	return nil
}

func (r *Resolver) VisitPrintStmt(stmt *ast.Print) any {
	r.resolveExpr(stmt.Expression)
	return nil
}

func (r *Resolver) VisitReturnStmt(stmt *ast.Return) any {
	if r.currentFunctionType == NOT_FUNC {
		globals.ReportError(stmt.Keyword.Line, "", "Can't return from top-level code.")
	}

	if stmt.Value != nil {
		if r.currentFunctionType == INITIALIZER {
			globals.ReportError(stmt.Keyword.Line, "", "Can't return a value from an initializer.")
		}
		r.resolveExpr(stmt.Value)
	}
	return nil
}

func (r *Resolver) VisitWhileStmt(stmt *ast.While) any {
	r.resolveExpr(stmt.Condition)
	r.resolveStmt(stmt.Body)
	return nil
}

func (r *Resolver) VisitBinaryExpr(expr *ast.Binary) any {
	r.resolveExpr(expr.Left)
	r.resolveExpr(expr.Right)
	return nil
}

func (r *Resolver) VisitCallExpr(expr *ast.Call) any {
	r.resolveExpr(expr.Callee)

	for _, arg := range expr.Arguments {
		r.resolveExpr(arg)
	}
	return nil
}

func (r *Resolver) VisitGroupingExpr(expr *ast.Grouping) any {
	r.resolveExpr(expr.Expression)
	return nil
}

func (r *Resolver) VisitLiteralExpr(expr *ast.Literal) any {
	return nil
}

func (r *Resolver) VisitLogicalExpr(expr *ast.Logical) any {
	r.resolveExpr(expr.Left)
	r.resolveExpr(expr.Right)
	return nil
}

func (r *Resolver) VisitUnaryExpr(expr *ast.Unary) any {
	r.resolveExpr(expr.Right)
	return nil
}

func (r *Resolver) VisitClassStmt(stmt *ast.Class) any {
	r.declare(stmt.Name)
	r.define(stmt.Name)

	r.beginScope()
	enclosingClass := r.currentClassType
	r.currentClassType = CLASS
	r.scopes[len(r.scopes)-1]["this"] = true

	for _, method := range stmt.Methods {
		declaration := METHOD
		if method.Name.Lexeme == "init" {
			declaration = INITIALIZER
		}
		r.resolveFunction(method, declaration)
	}
	r.currentClassType = enclosingClass
	r.endScope()

	return nil
}

func (r *Resolver) VisitGetExpr(expr *ast.Get) any {
	r.resolveExpr(expr.Object)
	return nil
}

func (r *Resolver) VisitSetExpr(expr *ast.Set) any {
	r.resolveExpr(expr.Value)
	r.resolveExpr(expr.Object)
	return nil
}

func (r *Resolver) VisitThisExpr(expr *ast.This) any {
	if r.currentClassType == NOT_CLASS {
		globals.ReportError(expr.Keyword.Line, "", "Can't use 'this' outside of a class.")
		return nil
	}
	r.resolveLocal(expr, expr.Keyword)
	return nil
}
