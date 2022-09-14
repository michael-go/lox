package main

import (
	"fmt"

	"github.com/michael-go/lox/golox/internal/ast"
	"github.com/michael-go/lox/golox/internal/token"
)

func main() {

	astPrinter := ast.AstPrinter{}

	expr := ast.Binary{
		Left: ast.Unary{
			Operator: token.Token{
				Type:    token.MINUS,
				Lexeme:  "-",
				Literal: nil,
				Line:    1,
			},
			Right: ast.Literal{
				Value: 123,
			},
		},
		Operator: token.Token{
			Type:    token.STAR,
			Lexeme:  "*",
			Literal: nil,
			Line:    1,
		},
		Right: ast.Grouping{
			Expression: ast.Literal{
				Value: 45.67,
			},
		},
	}

	fmt.Print(astPrinter.Print(expr))

}
