package parser

import (
	"fmt"
	"testing"

	"github.com/michael-go/lox/golox/internal/ast"
	"github.com/michael-go/lox/golox/internal/globals"
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
	statements := parser.Parse()
	if statements == nil {
		return "", nil
	} else {
		return fmt.Sprint(ast.AstPrinter{}.Print(statements)), nil
	}
}

func TestFoo(t *testing.T) {
	code := `1 + 2 * 3;`
	expected := `(; (+ 1 (* 2 3)))`
	actual, err := codeToAstString(code)
	assert.Nil(t, err)
	assert.Equal(t, expected, actual)
}

func TestComparisons(t *testing.T) {
	code := `"bar" != !!false < (3 / 2);`
	expected := `(; (!= bar (< (! (! false)) (group (/ 3 2)))))`
	actual, err := codeToAstString(code)
	assert.Nil(t, err)
	assert.Equal(t, expected, actual)
}

func TestParsingError(t *testing.T) {
	code := `$# foo;`
	expr, err := codeToAstString(code)
	assert.Nil(t, err)
	assert.Equal(t, "", expr)
	assert.True(t, globals.HadError)
}

func TestMissingCloseParenError(t *testing.T) {
	origReportError := globals.ReportError
	defer func() {
		globals.ReportError = origReportError
	}()

	errorReported := false
	globals.ReportError = func(line int, where string, message string) {
		assert.Equal(t, 1, line)
		assert.Equal(t, " at ';'", where)
		assert.Equal(t, "Expect ')' after expression.", message)
		errorReported = true
	}

	code := `1 + (2 * 3;`
	expr, err := codeToAstString(code)
	assert.Nil(t, err)
	assert.Equal(t, "", expr)
	assert.True(t, errorReported)
}
