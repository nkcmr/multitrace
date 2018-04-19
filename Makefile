PWD    := $(shell echo $$PWD)
GOPATH ?= $(shell go env GOPATH)

mtr: vendor/.ok
	GOPATH=$(GOPATH) \
		go build \
		-o $@ \
		github.com/nkcmr/multitrace/cmd/mtr
	sudo chown root $@
	sudo chmod u+s $@

vendor/.ok:
	dep ensure -vendor-only
	touch $@

.PHONY: clean
clean:
	rm -rf vendor
	rm -f ./mtr
