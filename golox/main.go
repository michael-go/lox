package main

import (
	"bufio"
	"fmt"
	"io"
	"io/ioutil"
	"os"

	"github.com/michael-go/lox/golox/internal/globals"
	"github.com/michael-go/lox/golox/internal/interpreter"
	"github.com/michael-go/lox/golox/internal/parser"
	"github.com/michael-go/lox/golox/internal/resolver"
	"github.com/michael-go/lox/golox/internal/scanner"
)

func run(interpreter *interpreter.Interpreter, source string) error {
	scan := scanner.New(source)
	tokens, err := scan.ScanTokens()
	if err != nil {
		return fmt.Errorf("faied to scan tokens: %w", err)
	}

	parser := parser.New(tokens)
	statements := parser.Parse()
	if globals.HadError {
		return fmt.Errorf("failed to parse")
	}

	resolver := resolver.New(interpreter)
	resolver.Resolve(statements)
	if globals.HadError {
		return fmt.Errorf("failed to resolve")
	}

	interpreter.Interpret(statements)
	return nil
}

func runFile(path string) error {
	content, err := ioutil.ReadFile(path)
	if err != nil {
		return fmt.Errorf("could not read file: %w", err)
	}

	interpreter := interpreter.New()

	run(&interpreter, string(content))

	return nil
}

func runPrompt() error {
	interpreter := interpreter.New()

	reader := bufio.NewReader(os.Stdin)

	for {
		fmt.Print("> ")
		line, err := reader.ReadString('\n')
		if err == io.EOF {
			break
		} else if err != nil {
			return fmt.Errorf("could not read line: %w", err)
		}
		run(&interpreter, line)
	}

	return nil
}

func main() {
	var err error

	if len(os.Args) > 2 {
		fmt.Println("Usage: golox [script]")
	} else if len(os.Args) == 2 {
		err = runFile(os.Args[1])
		if globals.HadError {
			os.Exit(65)
		} else if globals.HadRuntimeError {
			os.Exit(70)
		}
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
