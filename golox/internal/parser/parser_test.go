package parser

import (
	"fmt"
	"testing"

	"github.com/michael-go/go-jsn/jsn"
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
	json, err := jsn.NewJson(statements)
	if err != nil {
		return "", fmt.Errorf("failed to AST convert to json: %w", err)
	}
	if statements == nil || len(statements) == 0 {
		return "", nil
	} else {
		return json.Pretty(), nil
	}
}

func TestFoo(t *testing.T) {
	code := `1 + 2 * 3;`
	expected := `[
  {
    "Expression": {
      "Left": {
        "Value": 1
      },
      "Operator": {
        "Lexeme": "+",
        "Line": 1,
        "Literal": null,
        "Type": 7
      },
      "Right": {
        "Left": {
          "Value": 2
        },
        "Operator": {
          "Lexeme": "*",
          "Line": 1,
          "Literal": null,
          "Type": 10
        },
        "Right": {
          "Value": 3
        }
      }
    }
  }
]`
	actual, err := codeToAstString(code)
	assert.Nil(t, err)
	assert.Equal(t, expected, actual)
}

func TestComparisons(t *testing.T) {
	code := `"bar" != !!false < (3 / 2);`
	expected := `[
  {
    "Expression": {
      "Left": {
        "Value": "bar"
      },
      "Operator": {
        "Lexeme": "!=",
        "Line": 1,
        "Literal": null,
        "Type": 12
      },
      "Right": {
        "Left": {
          "Operator": {
            "Lexeme": "!",
            "Line": 1,
            "Literal": null,
            "Type": 11
          },
          "Right": {
            "Operator": {
              "Lexeme": "!",
              "Line": 1,
              "Literal": null,
              "Type": 11
            },
            "Right": {
              "Value": false
            }
          }
        },
        "Operator": {
          "Lexeme": "\u003c",
          "Line": 1,
          "Literal": null,
          "Type": 17
        },
        "Right": {
          "Expression": {
            "Left": {
              "Value": 3
            },
            "Operator": {
              "Lexeme": "/",
              "Line": 1,
              "Literal": null,
              "Type": 9
            },
            "Right": {
              "Value": 2
            }
          }
        }
      }
    }
  }
]`
	actual, err := codeToAstString(code)
	assert.Nil(t, err)
	assert.Equal(t, expected, actual)
}

func TestParsingError(t *testing.T) {
	code := `$# foo;`
	expr, err := codeToAstString(code)
	assert.Nil(t, err)
	assert.Equal(t, `[
  {
    "Expression": {
      "Name": {
        "Lexeme": "foo",
        "Line": 1,
        "Literal": null,
        "Type": 19
      }
    }
  }
]`, expr)
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
	assert.Equal(t, `[
  null
]`, expr)
	assert.True(t, errorReported)
}
