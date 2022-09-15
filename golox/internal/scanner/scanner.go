package scanner

import (
	"fmt"
	"strconv"
	"unicode/utf8"

	"github.com/michael-go/lox/golox/internal/globals"
	"github.com/michael-go/lox/golox/internal/token"
)

type Scanner struct {
	source string
	tokens []token.Token

	start   int
	current int
	line    int
}

func New(source string) Scanner {
	s := Scanner{source: source, line: 1}
	return s
}

func (s *Scanner) ScanTokens() ([]token.Token, error) {
	for !s.isAtEnd() {
		s.start = s.current
		s.scanToken()
	}

	s.start = s.current
	s.addToken(token.EOF)

	return s.tokens, nil
}

func (s *Scanner) isAtEnd() bool {
	return s.current >= len(s.source)
}

func (s *Scanner) scanToken() {
	r := s.advance()
	switch r {
	case rune('('):
		s.addToken(token.LEFT_PAREN)
	case rune(')'):
		s.addToken(token.RIGHT_PAREN)
	case rune('{'):
		s.addToken(token.LEFT_BRACE)
	case rune('}'):
		s.addToken(token.RIGHT_BRACE)
	case rune(','):
		s.addToken(token.COMMA)
	case rune('.'):
		s.addToken(token.DOT)
	case rune('-'):
		s.addToken(token.MINUS)
	case rune('+'):
		s.addToken(token.PLUS)
	case rune(';'):
		s.addToken(token.SEMICOLON)
	case rune('*'):
		s.addToken(token.STAR)
	case rune('!'):
		if s.match('=') {
			s.addToken(token.BANG_EQUAL)
		} else {
			s.addToken(token.BANG)
		}
	case rune('='):
		if s.match('=') {
			s.addToken(token.EQUAL_EQUAL)
		} else {
			s.addToken(token.EQUAL)
		}
	case rune('<'):
		if s.match('=') {
			s.addToken(token.LESS_EQUAL)
		} else {
			s.addToken(token.LESS)
		}
	case rune('>'):
		if s.match('=') {
			s.addToken(token.GREATER_EQUAL)
		} else {
			s.addToken(token.GREATER)
		}
	case rune('/'):
		if s.match('/') {
			for !s.isAtEnd() && s.peek() != '\n' {
				s.advance()
			}
		} else {
			s.addToken(token.SLASH)
		}
	case rune(' '):
	case rune('\r'):
	case rune('\t'):
	case rune('\n'):
		s.line++
	case rune('"'):
		s.string()
	default:
		if isDigit(r) {
			s.number()
		} else if isAlpha(r) {
			s.identifier()
		} else {
			globals.ReportError(s.line, "", fmt.Sprintf("Unexpected character %#U", r))
		}
	}
}

func (s *Scanner) advance() rune {
	r, len := utf8.DecodeRuneInString(s.source[s.current:])
	s.current += len
	return r
}

func (s *Scanner) addToken(t token.Type) {
	s.addTokenLiteral(t, nil)
}

func (s *Scanner) addTokenLiteral(tokenType token.Type, literal any) {
	text := s.source[s.start:s.current]
	s.tokens = append(s.tokens, token.Token{Type: tokenType, Lexeme: text, Literal: literal, Line: s.line})
}

func (s *Scanner) match(expected rune) bool {
	if s.isAtEnd() {
		return false
	}

	r, _ := utf8.DecodeRuneInString(s.source[s.current:])
	if r != expected {
		return false
	}

	s.advance()
	return true
}

func (s *Scanner) peek() rune {
	if s.isAtEnd() {
		return rune(0)
	}

	r, _ := utf8.DecodeRuneInString(s.source[s.current:])
	return r
}

func (s *Scanner) peekNext() rune {
	if s.current+1 >= len(s.source) {
		return rune(0)
	}

	r, _ := utf8.DecodeRuneInString(s.source[s.current+1:])
	return r
}

func (s *Scanner) string() {
	for !s.isAtEnd() && s.peek() != '"' {
		if s.peek() == '\n' {
			s.line++
		}
		s.advance()
	}

	if s.isAtEnd() {
		globals.ReportError(s.line, "", "Unterminated string.")
		return
	}

	s.advance()

	value := s.source[s.start+1 : s.current-1]
	s.addTokenLiteral(token.STRING, value)
}

func (s *Scanner) number() {
	for isDigit(s.peek()) {
		s.advance()
	}

	if s.peek() == '.' && isDigit(s.peekNext()) {
		s.advance()
		for isDigit(s.peek()) {
			s.advance()
		}
	}

	value, _ := strconv.ParseFloat(s.source[s.start:s.current], 64)
	s.addTokenLiteral(token.NUMBER, value)
}

func (s *Scanner) identifier() {
	for isAlphaNumeric(s.peek()) {
		s.advance()
	}

	text := s.source[s.start:s.current]
	tokenType, exists := keywords[text]
	if !exists {
		tokenType = token.IDENTIFIER
	}
	s.addToken(tokenType)
}
