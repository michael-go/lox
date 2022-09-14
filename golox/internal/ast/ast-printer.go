package ast

import (
	"fmt"
	"strings"
)

type AstPrinter struct {
}

func (p AstPrinter) Print(expr Expr[string]) string {
	return expr.Accept(p)
}

func (p AstPrinter) VisitBinaryExpr(expr Binary[string]) string {
	return p.parenthesize(expr.Operator.Lexeme, expr.Left, expr.Right)
}

func (p AstPrinter) VisitGroupingExpr(expr Grouping[string]) string {
	return p.parenthesize("group", expr.Expression)
}

func (p AstPrinter) VisitLiteralExpr(expr Literal[string]) string {
	if expr.Value == nil {
		return "nil"
	}
	return fmt.Sprint(expr.Value)
}

func (p AstPrinter) VisitUnaryExpr(expr Unary[string]) string {
	return p.parenthesize(expr.Operator.Lexeme, expr.Right)
}

func (p AstPrinter) parenthesize(name string, exprs ...Expr[string]) string {
	var builder strings.Builder

	builder.WriteString("(")
	builder.WriteString(name)

	for _, expr := range exprs {
		builder.WriteString(" ")
		builder.WriteString(expr.Accept(p))
	}

	builder.WriteString(")")

	return builder.String()
}
