// Code generated by generate-ast. DO NOT EDIT.
package ast

import "github.com/michael-go/lox/golox/internal/token"

var _ = token.Token{} // to avoid unused import error

type Stmt interface {
	Accept(visitor StmtVisitor) any
}

type Block struct {
	Statements []Stmt
}

type Expression struct {
	Expression Expr
}

type If struct {
	Condition  Expr
	ThenBranch Stmt
	ElseBranch Stmt
}

type Print struct {
	Expression Expr
}

type Var struct {
	Name        token.Token
	Initializer Expr
}

type While struct {
	Condition Expr
	Body      Stmt
}

type StmtVisitor interface {
	VisitBlockStmt(stmt Block) any
	VisitExpressionStmt(stmt Expression) any
	VisitIfStmt(stmt If) any
	VisitPrintStmt(stmt Print) any
	VisitVarStmt(stmt Var) any
	VisitWhileStmt(stmt While) any
}

func (stmt Block) Accept(visitor StmtVisitor) any {
	return visitor.VisitBlockStmt(stmt)
}

func (stmt Expression) Accept(visitor StmtVisitor) any {
	return visitor.VisitExpressionStmt(stmt)
}

func (stmt If) Accept(visitor StmtVisitor) any {
	return visitor.VisitIfStmt(stmt)
}

func (stmt Print) Accept(visitor StmtVisitor) any {
	return visitor.VisitPrintStmt(stmt)
}

func (stmt Var) Accept(visitor StmtVisitor) any {
	return visitor.VisitVarStmt(stmt)
}

func (stmt While) Accept(visitor StmtVisitor) any {
	return visitor.VisitWhileStmt(stmt)
}
