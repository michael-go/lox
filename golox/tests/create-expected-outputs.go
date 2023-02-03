package main

import (
	"fmt"
	"io/ioutil"
	"os/exec"
	"strings"
)

func createExpectedOutputs() {
	const dirPrefix = "tests/fixtures/"
	fileInfos, err := ioutil.ReadDir(dirPrefix)
	if err != nil {
		panic(fmt.Errorf("could not read fixtures directory: %v", err))
	}
	for _, fileInfo := range fileInfos {
		if strings.HasSuffix(fileInfo.Name(), ".lox") {
			loxPath := dirPrefix + fileInfo.Name()
			expectedPath := dirPrefix + strings.TrimSuffix(fileInfo.Name(), ".lox") + ".out"

			fmt.Println("Generating ", expectedPath)

			cmd := exec.Command("go", "run", "main.go", loxPath)
			stdout, err := cmd.Output()
			stderr := ""
			if err != nil {
				exitError, ok := err.(*exec.ExitError)
				if !ok {
					panic(fmt.Errorf("failed to run lox file %s, err: %v", loxPath, err))
				}
				stderr = string(exitError.Stderr)
			}

			var expected strings.Builder
			expected.WriteString(fmt.Sprintf("# exit code: %d\n", cmd.ProcessState.ExitCode()))
			expected.WriteString(fmt.Sprintf("# stdout:\n%s\n", string(stdout)))
			expected.WriteString(fmt.Sprintf("# stderr:\n%s\n", string(stderr)))

			err = ioutil.WriteFile(expectedPath, []byte(expected.String()), 0644)
			if err != nil {
				panic(fmt.Errorf("could not write expected output: %v", err))
			}
		}
	}
}

func main() {
	createExpectedOutputs()
}
