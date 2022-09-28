package main

import (
	"fmt"
	"io/ioutil"
	"os"

	"github.com/michael-go/lox/golox/internal/globals"
	"github.com/michael-go/lox/golox/internal/parser"
	"github.com/michael-go/lox/golox/internal/scanner"

	"github.com/michael-go/go-jsn/jsn"
)

func printAst(source string) error {
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

	json, err := jsn.NewJson(statements)
	if err != nil {
		return fmt.Errorf("failed to AST convert to json: %w", err)
	}
	fmt.Println(json.Pretty())

	return nil
}

func main() {
	if len(os.Args) != 2 {
		fmt.Println("Usage: print-ast [lox source file]")
		os.Exit(1)
	}

	sourceFile := os.Args[1]
	source, err := ioutil.ReadFile(sourceFile)
	if err != nil {
		fmt.Println("Could not read file:", err)
		os.Exit(1)
	}

	err = printAst(string(source))
	if err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}
