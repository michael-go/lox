package parser

import (
	"github.com/michael-go/lox/golox/internal/ast"
	"github.com/michael-go/lox/golox/internal/globals"
	"github.com/michael-go/lox/golox/internal/token"
)

type Parser struct {
	tokens  []token.Token
	current int
}

type ParserError struct {
	message string
}

func New(tokens []token.Token) Parser {
	return Parser{tokens: tokens}
}

func (p *Parser) Parse() []ast.Stmt {
	var statements []ast.Stmt
	for !p.isAtEnd() {
		statements = append(statements, p.decleration())
	}

	return statements
}

func (p *Parser) decleration() ast.Stmt {
	recorver := func() {
		if r := recover(); r != nil {
			_, ok := r.(ParserError)
			if !ok {
				panic(r)
			}
			p.synchronize()
			// TODO: it can return null, so need to make sure interprerter can handle it
		}
	}
	defer recorver()

	if p.match(token.VAR) {
		return p.varDecleration()
	}

	return p.statement()
}

func (p *Parser) varDecleration() ast.Stmt {
	name := p.consume(token.IDENTIFIER, "Expect variable name.")

	var initializer ast.Expr
	if p.match(token.EQUAL) {
		initializer = p.expression()
	}

	p.consume(token.SEMICOLON, "Expect ';' after variable declaration.")
	return ast.Var{Name: name, Initializer: initializer}
}

func (p *Parser) statement() ast.Stmt {
	if p.match(token.PRINT) {
		return p.printStatement()
	}

	if p.match(token.LEFT_BRACE) {
		return ast.Block{Statements: p.block()}
	}

	return p.expressionStatement()
}

func (p *Parser) block() []ast.Stmt {
	var statements []ast.Stmt

	for !p.check(token.RIGHT_BRACE) && !p.isAtEnd() {
		statements = append(statements, p.decleration())
	}

	p.consume(token.RIGHT_BRACE, "Expect '}' after block.")
	return statements
}

func (p *Parser) printStatement() ast.Stmt {
	value := p.expression()
	p.consume(token.SEMICOLON, "Expect ';' after value.")
	return ast.Print{Expression: value}
}

func (p *Parser) expressionStatement() ast.Stmt {
	expr := p.expression()
	p.consume(token.SEMICOLON, "Expect ';' after expression.")
	return ast.Expression{Expression: expr}
}

func (p *Parser) expression() ast.Expr {
	return p.assignment()
}

func (p *Parser) assignment() ast.Expr {
	expr := p.equality()

	if p.match(token.EQUAL) {
		equals := p.previous()
		value := p.assignment()

		if name, ok := expr.(ast.Variable); ok {
			return ast.Assign{Name: name.Name, Value: value}
		}

		p.panicError(equals, "Invalid assignment target.")
	}

	return expr
}

func (p *Parser) equality() ast.Expr {
	expr := p.comparison()

	for p.match(token.BANG_EQUAL, token.EQUAL_EQUAL) {
		operator := p.previous()
		right := p.comparison()
		expr = ast.Binary{Left: expr, Operator: operator, Right: right}
	}

	return expr
}

func (p *Parser) comparison() ast.Expr {
	expr := p.term()

	for p.match(token.GREATER, token.GREATER_EQUAL, token.LESS, token.LESS_EQUAL) {
		operator := p.previous()
		right := p.term()
		expr = ast.Binary{Left: expr, Operator: operator, Right: right}
	}

	return expr
}

func (p *Parser) term() ast.Expr {
	expr := p.factor()

	for p.match(token.MINUS, token.PLUS) {
		operator := p.previous()
		right := p.factor()
		expr = ast.Binary{Left: expr, Operator: operator, Right: right}
	}

	return expr
}

func (p *Parser) factor() ast.Expr {
	expr := p.unary()

	for p.match(token.SLASH, token.STAR) {
		operator := p.previous()
		right := p.unary()
		expr = ast.Binary{Left: expr, Operator: operator, Right: right}
	}

	return expr
}

func (p *Parser) unary() ast.Expr {
	if p.match(token.BANG, token.MINUS) {
		operator := p.previous()
		right := p.unary()
		return ast.Unary{Operator: operator, Right: right}
	}

	return p.primary()
}

func (p *Parser) primary() ast.Expr {
	if p.match(token.FALSE) {
		return ast.Literal{Value: false}
	}
	if p.match(token.TRUE) {
		return ast.Literal{Value: true}
	}
	if p.match(token.NIL) {
		return ast.Literal{Value: nil}
	}

	if p.match(token.NUMBER, token.STRING) {
		return ast.Literal{Value: p.previous().Literal}
	}

	if p.match(token.IDENTIFIER) {
		return ast.Variable{Name: p.previous()}
	}

	if p.match(token.LEFT_PAREN) {
		expr := p.expression()
		p.consume(token.RIGHT_PAREN, "Expect ')' after expression.")
		return ast.Grouping{Expression: expr}
	}

	p.panicError(p.peek(), "Expect expression.")
	return nil
}

func (p *Parser) consume(tokenType token.Type, message string) token.Token {
	if p.check(tokenType) {
		return p.advance()
	}

	p.panicError(p.peek(), message)
	return token.Token{}
}

func (p *Parser) panicError(t token.Token, message string) {
	p.reportError(p.peek(), message)
	panic(ParserError{message: message})
}

func (p *Parser) reportError(t token.Token, message string) {
	if t.Type == token.EOF {
		globals.ReportError(t.Line, " at end", message)
	} else {
		globals.ReportError(t.Line, " at '"+t.Lexeme+"'", message)
	}
}

func (p *Parser) match(types ...token.Type) bool {
	for _, t := range types {
		if p.check(t) {
			p.advance()
			return true
		}
	}
	return false
}

func (p *Parser) check(tokenType token.Type) bool {
	if p.isAtEnd() {
		return false
	}
	return p.peek().Type == tokenType
}

func (p *Parser) advance() token.Token {
	if !p.isAtEnd() {
		p.current++
	}
	return p.previous()
}

func (p *Parser) isAtEnd() bool {
	return p.current == len(p.tokens) || p.peek().Type == token.EOF
}

func (p *Parser) peek() token.Token {
	return p.tokens[p.current]
}

func (p *Parser) previous() token.Token {
	return p.tokens[p.current-1]
}

func (p *Parser) synchronize() {
	p.advance()

	for !p.isAtEnd() {
		if p.previous().Type == token.SEMICOLON {
			return
		}

		switch p.peek().Type {
		case token.CLASS:
		case token.FUN:
		case token.VAR:
		case token.FOR:
		case token.IF:
		case token.WHILE:
		case token.PRINT:
		case token.RETURN:
			return
		}

		p.advance()
	}
}
