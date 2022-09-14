package main

import (
	"fmt"

	"github.com/michael-go/lox/golox/internal/ast"
	"github.com/michael-go/lox/golox/internal/token"
)

func main() {

	astPrinter := ast.AstPrinter{}

	expr := ast.Binary[string]{
		Left: ast.Unary[string]{
			Operator: token.Token{
				Type:    token.MINUS,
				Lexeme:  "-",
				Literal: nil,
				Line:    1,
			},
			Right: ast.Literal[string]{
				Value: 123,
			},
		},
		Operator: token.Token{
			Type:    token.STAR,
			Lexeme:  "*",
			Literal: nil,
			Line:    1,
		},
		Right: ast.Grouping[string]{
			Expression: ast.Literal[string]{
				Value: 45.67,
			},
		},
	}

	fmt.Print(astPrinter.Print(expr))

}
