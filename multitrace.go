package multitrace

import (
	"context"
	// "fmt"
	"log"
	"math/rand"
	"net"
	"os"
	"sync/atomic"
	"time"

	"golang.org/x/net/icmp"
	"golang.org/x/net/ipv4"
)

type IPKind int

type proto int

const (
	IPv4 IPKind = iota
	IPv6
)

const (
	protoICMP   proto = 1
	protoTCP          = 6
	protoUDP          = 17
	protoICMPv6       = 58
)

const defaultMaxHops = 30

type Option func(*Multitracer)

func Context(ctx context.Context) Option {
	return Option(func(m *Multitracer) {
		m.ctx = ctx
	})
}

func UseUDP() Option {
	return Option(func(m *Multitracer) {
		m.proto = protoUDP
	})
}

func UseTCP() Option {
	return Option(func(m *Multitracer) {
		m.proto = protoTCP
	})
}

func UseICMP() Option {
	return Option(func(m *Multitracer) {
		m.proto = protoICMP
	})
}

func UseIPv4() Option {
	return Option(func(m *Multitracer) {
		m.ipk = IPv4
	})
}

func Timeout(t time.Duration) Option {
	return Option(func(m *Multitracer) {
		m.timeout = t
	})
}

type Multitracer struct {
	hostname string
	maxHops  int
	ctx      context.Context
	ipk      IPKind
	proto    proto
	rand     rand.Source
	timeout  time.Duration
}

func NewMultitracer(hostname string, options ...Option) (*Multitracer, error) {
	m := &Multitracer{
		hostname: hostname,
		maxHops:  defaultMaxHops,
		ctx:      context.Background(),
		ipk:      IPv4,
		proto:    protoICMP,
		rand:     rand.NewSource(time.Now().UnixNano()),
		timeout:  time.Second * 5,
	}
	for _, opt := range options {
		opt(m)
	}
	return m, nil
}

func pid() int {
	return os.Getpid() & 0xffff
}

var globSeq int32 = 0

func seq() int {
	return int(atomic.AddInt32(&globSeq, 1))
}

func (m *Multitracer) Run() error {
	c, err := icmp.ListenPacket("ip4:icmp", "0.0.0.0")
	if err != nil {
		return err
	}
	defer c.Close()

	ttl := 0
	for {
		ttl++
		err, done, addr := m.emitIcmp(c, ttl)
		if err != nil {
			return err
		}
		var h string
		if addr == nil {
			h = "???"
		} else {
			h = addr.String()
		}
		log.Printf("hop: %s (done: %t)\n", h, done)
		if done {
			break
		}
	}

	return nil
}

func (m *Multitracer) emitIcmp(c *icmp.PacketConn, ttl int) (error, bool, net.Addr) {
	defer mustSetTTL(c.IPv4PacketConn(), m.maxHops)
	mustSetTTL(c.IPv4PacketConn(), ttl)
	mustSetDeadline(c, time.Now().Add(m.timeout))
	ereq := icmp.Message{
		Type: ipv4.ICMPTypeEcho,
		Code: 0,
		Body: &icmp.Echo{
			ID:   pid(),
			Seq:  seq(),
			Data: []byte{},
		},
	}
	edata, err := ereq.Marshal(nil)
	if err != nil {
		return err, false, nil
	}
	if _, err := c.WriteTo(edata, &net.IPAddr{IP: net.ParseIP("8.8.8.8")}); err != nil {
		return err, false, nil
	}
	for {
		b := make([]byte, 1500)
		_, addr, err := c.ReadFrom(b)
		if err != nil {
			if nerr, ok := err.(*net.OpError); ok {
				return nil, !nerr.Timeout(), nil
			}
			return err, false, nil
		}
		msg, err := icmp.ParseMessage(int(protoICMP), b)
		if err != nil {
			return err, false, nil
		}
		switch bod := msg.Body.(type) {
		case *icmp.Echo:
			log.Println("received echo reply packet")
			if bod.ID != pid() {
				log.Printf("ignoring echo reply packet %d != %d", bod.ID, pid())
				continue
			}
			return nil, true, addr
		case *icmp.TimeExceeded:
			log.Println("received time exceeded packet")
			{
				origPacket, err := icmp.ParseMessage(int(protoICMP), bod.Data[20:])
				if err != nil {
					return err, false, nil
				}
				id := origPacket.Body.(*icmp.Echo).ID
				if id != pid() {
					log.Printf("ignoring time exceeded packet %d != %d", id, pid())
					continue
				}
				return nil, false, addr
			}
		}
	}
}

func mustSetTTL(c interface{ SetTTL(int) error }, ttl int) {
	err := c.SetTTL(ttl)
	if err != nil {
		panic("error occured while setting socket ttl: " + err.Error())
	}
}

func mustSetDeadline(c interface{ SetDeadline(time.Time) error }, d time.Time) {
	err := c.SetDeadline(d)
	if err != nil {
		panic("error occured while setting socket deadline: " + err.Error())
	}
}

// type EventHandler interface {
// 	OnHost(hop int, host net.IP) error
// 	OnEmit(hop, seq int, ts int64) error
// 	OnPing(hop int, rt int64, seq int) error
// 	OnDrop(hop, seq int) error
// 	OnDNS(hop int, hostname string) error
// }
