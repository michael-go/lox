package parser

import (
	"fmt"
	"testing"

	"github.com/michael-go/lox/golox/internal/ast"
	"github.com/michael-go/lox/golox/internal/scanner"
	"github.com/stretchr/testify/assert"
)

func codeToAstString(code string) (string, error) {
	scan := scanner.New(code)
	tokens, err := scan.ScanTokens()
	if err != nil {
		return "", fmt.Errorf("faied to scan tokens: %w", err)
	}

	parser := New(tokens)
	expr := parser.Parse()
	if expr == nil {
		return "", nil
	} else {
		return fmt.Sprint(ast.AstPrinter{}.Print(expr)), nil
	}
}

func TestFoo(t *testing.T) {
	code := `1 + 2 * 3`
	expected := `(+ 1 (* 2 3))`
	actual, err := codeToAstString(code)
	assert.Nil(t, err)
	assert.Equal(t, expected, actual)
}

func TestComparisons(t *testing.T) {
	code := `"bar" != !!false < (3 / 2)`
	expected := `(!= bar (< (! (! false)) (group (/ 3 2))))`
	actual, err := codeToAstString(code)
	assert.Nil(t, err)
	assert.Equal(t, expected, actual)
}

func TestParsingError(t *testing.T) {
	code := `$# foo`
	expr, err := codeToAstString(code)
	assert.Nil(t, err)
	assert.Equal(t, "", expr)
}
