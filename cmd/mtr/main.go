package main

import (
	"flag"
	"log"
	"os"
	"time"

	"github.com/nkcmr/multitrace"
)

type cmdOptions struct {
	target  string
	timeout time.Duration
}

func initflagset() (*flag.FlagSet, *cmdOptions) {
	o := new(cmdOptions)
	fs := flag.NewFlagSet("mtr", flag.ContinueOnError)
	fs.StringVar(&o.target, "target", "127.0.0.1", "the end hostname or address")
	fs.DurationVar(&o.timeout, "timeout", time.Second*30, "")
	return fs, o
}

func main() {
	os.Exit(_main())
}

func _main() int {
	fs, opts := initflagset()
	err := fs.Parse(os.Args[1:])
	if err != nil {
		if err != flag.ErrHelp {
			log.Printf("error: %s", err)
		}
		return 1
	}
	mtr, err := multitrace.NewMultitracer(opts.target, multitrace.Timeout(opts.timeout))
	if err != nil {
		log.Printf("error: %s", err)
		return 3
	}
	if err := mtr.Run(); err != nil {
		log.Printf("error: %s", err)
		return 2
	}

	return 0
}
