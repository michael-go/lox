package globals

import "fmt"

var HadError bool

func ReportError(line int, where string, message string) {
	fmt.Println(fmt.Sprintf("[line %d] Error: %s: %s", line, where, message))
	HadError = true
}
