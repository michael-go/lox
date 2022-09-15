package main

import (
	"bufio"
	"fmt"
	"io"
	"io/ioutil"
	"os"

	"github.com/michael-go/lox/golox/internal/ast"
	"github.com/michael-go/lox/golox/internal/parser"
	"github.com/michael-go/lox/golox/internal/scanner"
)

func run(source string) error {
	scan := scanner.New(source)
	tokens, err := scan.ScanTokens()
	if err != nil {
		return fmt.Errorf("faied to scan tokens: %w", err)
	}

	parser := parser.New(tokens)
	expr := parser.Parse()
	fmt.Println(ast.AstPrinter{}.Print(expr))

	return nil
}

func runFile(path string) error {
	content, err := ioutil.ReadFile(path)
	if err != nil {
		return fmt.Errorf("could not read file: %w", err)
	}

	return run(string(content))
}

func runPrompt() error {
	reader := bufio.NewReader(os.Stdin)

	for {
		fmt.Print("> ")
		line, err := reader.ReadString('\n')
		if err == io.EOF {
			break
		} else if err != nil {
			return fmt.Errorf("could not read line: %w", err)
		}
		err = run(line)
		if err != nil {
			return fmt.Errorf("%w", err)
		}
	}

	return nil
}

func main() {
	fmt.Println(os.Args)

	var err error

	if len(os.Args) > 2 {
		fmt.Println("Usage: golox [script]")
	} else if len(os.Args) == 2 {
		err = runFile(os.Args[1])
	} else {
		err = runPrompt()
	}

	if err != nil {
		fmt.Println(err)
		os.Exit(1)
	} else {
		os.Exit(0)
	}
}
