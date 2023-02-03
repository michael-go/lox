package globals

import (
	"fmt"
	"os"

	"github.com/michael-go/lox/golox/internal/token"
)

type RuntimeError struct {
	Token   token.Token
	Message string
}

var HadError bool
var HadRuntimeError bool

var ReportError = func(line int, where string, message string) {
	fmt.Fprintln(os.Stderr, fmt.Sprintf("[line %d] Error: %s: %s", line, where, message))
	HadError = true
}

var ReportRuntimeError = func(err RuntimeError) {
	fmt.Fprintln(os.Stderr, fmt.Sprintf("%s\n[line %d]", err.Message, err.Token.Line))
	HadRuntimeError = true
}
