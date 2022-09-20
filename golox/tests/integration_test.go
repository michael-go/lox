package tests

import (
	"fmt"
	"io/ioutil"
	"os/exec"
	"regexp"
	"strconv"
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
)

func parseExpected(expectedPath string) (int, string, error) {
	expected, err := ioutil.ReadFile(expectedPath)
	if err != nil {
		return 0, "", fmt.Errorf("could not read expected output: %w", err)
	}

	r := regexp.MustCompile(`(?s)# exit code: (?P<ExitCode>\d+)\n# stdout:\n(?P<Stdout>.*)`)
	match := r.FindStringSubmatch(string(expected))
	exitCode, err := strconv.Atoi(match[1])
	if err != nil {
		return 0, "", fmt.Errorf("could not parse exit code: %w", err)
	}
	stdout := match[2]

	return exitCode, stdout, nil
}

func TestIntegration(t *testing.T) {
	fileInfos, err := ioutil.ReadDir("fixtures")
	if err != nil {
		t.Fatalf("could not read fixtures directory: %v", err)
	}
	var testsCount int
	for _, fileInfo := range fileInfos {
		if strings.HasSuffix(fileInfo.Name(), ".lox") {
			testsCount++

			testName := strings.TrimSuffix(fileInfo.Name(), ".lox")
			loxPath := "fixtures/" + fileInfo.Name()
			expectedPath := "fixtures/" + strings.TrimSuffix(fileInfo.Name(), ".lox") + ".out"
			t.Run(testName, func(t *testing.T) {
				cmd := exec.Command("go", "run", "../main.go", loxPath)
				stdout, err := cmd.Output()
				if _, ok := err.(*exec.ExitError); err != nil && !ok {
					t.Fatalf("failed to run lox file %s, err: %v", loxPath, err)
				}

				expectedExitCode, expectedStdout, err := parseExpected(expectedPath)
				if err != nil {
					t.Fatalf("could not parse expected output: %v", err)
				}
				assert.Equal(t, expectedExitCode, cmd.ProcessState.ExitCode())
				assert.Equal(t, expectedStdout, string(stdout))
			})
		}
	}

	assert.Greater(t, testsCount, 0)
}
