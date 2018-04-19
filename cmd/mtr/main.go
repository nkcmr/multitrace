package main

import (
	"fmt"
	"os"

	"github.com/nkcmr/multitrace"
)

func main() {
	mtr, _ := multitrace.NewMultitracer("8.8.8.8")
	if err := mtr.Run(); err != nil {
		fmt.Fprintf(os.Stderr, "error: %s", err.Error())
	}
}
