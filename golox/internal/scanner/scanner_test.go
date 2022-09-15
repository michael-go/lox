package scanner

import (
	"fmt"
	"testing"

	"github.com/michael-go/lox/golox/internal/globals"
	"github.com/michael-go/lox/golox/internal/token"
	"github.com/stretchr/testify/assert"
)

func TestFoo(t *testing.T) {
	t.Log("foo")
}

func tokensString(tokens []token.Token) string {
	var str string
	for _, token := range tokens {
		str += fmt.Sprintln(token)
	}
	return str
}

func TestNumbers(t *testing.T) {
	scanner := New("(13.37 + 18) * -7")
	tokens, err := scanner.ScanTokens()
	assert.Nil(t, err)
	assert.False(t, globals.HadError)
	assert.Equal(t, []token.Token{
		{Type: token.LEFT_PAREN, Lexeme: "(", Line: 1},
		{Type: token.NUMBER, Lexeme: "13.37", Literal: 13.37, Line: 1},
		{Type: token.PLUS, Lexeme: "+", Line: 1},
		{Type: token.NUMBER, Lexeme: "18", Literal: 18.0, Line: 1},
		{Type: token.RIGHT_PAREN, Lexeme: ")", Line: 1},
		{Type: token.STAR, Lexeme: "*", Line: 1},
		{Type: token.MINUS, Lexeme: "-", Line: 1},
		{Type: token.NUMBER, Lexeme: "7", Literal: 7.0, Line: 1},
		{Type: token.EOF, Line: 1},
	}, tokens)
}

func TestMultiline(t *testing.T) {
	scanner := New(`
		for (var i = 0; i < 10; i = i + 1) {
			foo(i)
			print i
		}
		`)
	tokens, err := scanner.ScanTokens()
	assert.Nil(t, err)
	assert.False(t, globals.HadError)
	tokensStr := tokensString(tokens)
	assert.Equal(t, `FOR for <nil>
LEFT_PAREN ( <nil>
VAR var <nil>
IDENTIFIER i <nil>
EQUAL = <nil>
NUMBER 0 0
SEMICOLON ; <nil>
IDENTIFIER i <nil>
LESS < <nil>
NUMBER 10 10
SEMICOLON ; <nil>
IDENTIFIER i <nil>
EQUAL = <nil>
IDENTIFIER i <nil>
PLUS + <nil>
NUMBER 1 1
RIGHT_PAREN ) <nil>
LEFT_BRACE { <nil>
IDENTIFIER foo <nil>
LEFT_PAREN ( <nil>
IDENTIFIER i <nil>
RIGHT_PAREN ) <nil>
PRINT print <nil>
IDENTIFIER i <nil>
RIGHT_BRACE } <nil>
EOF  <nil>
`,
		tokensStr)
}

func TestErrors(t *testing.T) {
	scanner := New("$?x")
	tokens, err := scanner.ScanTokens()
	assert.Nil(t, err)
	assert.True(t, globals.HadError)
	assert.Equal(t, []token.Token{
		{Type: token.IDENTIFIER, Lexeme: "x", Line: 1},
		{Type: token.EOF, Line: 1},
	}, tokens)
}
