package interpreter

import (
	"testing"

	"github.com/michael-go/lox/golox/internal/globals"
	"github.com/michael-go/lox/golox/internal/parser"
	"github.com/michael-go/lox/golox/internal/scanner"
	"github.com/stretchr/testify/assert"
)

func interpret(t *testing.T, code string) string {
	scan := scanner.New(code)
	tokens, err := scan.ScanTokens()
	if err != nil {
		t.Fatalf("faied to scan tokens: %v", err)
		return ""
	}

	parser := parser.New(tokens)
	expr := parser.Parse()
	if expr == nil {
		t.Fatalf("failed to parse %v", err)
		return ""
	}

	interpreter := New()
	return interpreter.Interpret(expr)
}

func TestCalc(t *testing.T) {
	result := interpret(t, `1 + 2 * 3`)
	assert.Equal(t, "7", result)

	result = interpret(t, `(1 + 2) * 3`)
	assert.Equal(t, "9", result)

	result = interpret(t, `1 + 2 * 3 - 4 / 5`)
	assert.Equal(t, "6.2", result)
}

func TestComp(t *testing.T) {
	assert.Equal(t, "true", interpret(t, `1 < 2`))

	assert.Equal(t, "false", interpret(t, `2 < 2`))

	assert.Equal(t, "true", interpret(t, `2 <= 2`))

	assert.Equal(t, "true", interpret(t, `"foo" == "fo" + "o"`))

	assert.Equal(t, "false", interpret(t, `"foo" == "bar"`))

	assert.Equal(t, "true", interpret(t, `7 == (3 + 4)`))

	// TODO: ensure that this is correct
	assert.Equal(t, "false", interpret(t, `true == 7 == 7`))

	assert.Equal(t, "true", interpret(t, `true == (7 == 7)`))
}

func TestRuntimeError(t *testing.T) {
	defer func() {
		globals.HadRuntimeError = false
	}()
	globals.HadRuntimeError = false

	result := interpret(t, `-"foo"`)
	assert.Equal(t, "", result)
	assert.True(t, globals.HadRuntimeError)
}

func TestRuntimeErrorMessage(t *testing.T) {
	origReportRuntimeError := globals.ReportRuntimeError
	defer func() {
		globals.ReportRuntimeError = origReportRuntimeError
	}()

	errorReported := false
	globals.ReportRuntimeError = func(err globals.RuntimeError) {
		errorReported = true
		assert.Equal(t, "Operands must be two numbers or two strings.", err.Message)
	}

	interpret(t, `1 + "foo"`)
	assert.True(t, errorReported)
}
