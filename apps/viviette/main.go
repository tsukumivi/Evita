package main

import (
	"os"

	"viviette/internal/cli"
)

func main() {
	os.Exit(cli.Main(os.Args[1:]))
}
