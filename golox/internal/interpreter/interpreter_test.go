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
	statements := parser.Parse()
	if statements == nil {
		t.Fatalf("failed to parse")
		return ""
	}

	interpreter := New()
	var result string
	interpreter.Print = func(str string) {
		result = result + str
	}
	interpreter.Interpret(statements)
	return result
}

func TestCalc(t *testing.T) {
	result := interpret(t, `print 1 + 2 * 3;`)
	assert.Equal(t, "7\n", result)

	result = interpret(t, `print (1 + 2) * 3;`)
	assert.Equal(t, "9\n", result)

	result = interpret(t, `print 1 + 2 * 3 - 4 / 5;`)
	assert.Equal(t, "6.2\n", result)
}

func TestComp(t *testing.T) {
	assert.Equal(t, "true\n", interpret(t, `print 1 < 2;`))

	assert.Equal(t, "false\n", interpret(t, `print 2 < 2;`))

	assert.Equal(t, "true\n", interpret(t, `print 2 <= 2;`))

	assert.Equal(t, "true\n", interpret(t, `print "foo" == "fo" + "o";`))

	assert.Equal(t, "false\n", interpret(t, `print "foo" == "bar";`))

	assert.Equal(t, "true\n", interpret(t, `print 7 == (3 + 4);`))

	// TODO: ensure that this is correct
	assert.Equal(t, "false\n", interpret(t, `print true == 7 == 7;`))

	assert.Equal(t, "true\n", interpret(t, `print true == (7 == 7);`))
}

func TestRuntimeError(t *testing.T) {
	defer func() {
		globals.HadRuntimeError = false
	}()
	globals.HadRuntimeError = false

	result := interpret(t, `-"foo";`)
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

	interpret(t, `print 1 + "foo";`)
	assert.True(t, errorReported)
}
