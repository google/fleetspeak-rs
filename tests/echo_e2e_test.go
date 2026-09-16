package echo_e2e_test

import (
	"bytes"
	"context"
	"errors"
	"flag"
	"io"
	"testing"
	"time"

	fsclient "github.com/google/fleetspeak/fleetspeak/src/client"
	fscomms "github.com/google/fleetspeak/fleetspeak/src/client/comms"
	fsconfig "github.com/google/fleetspeak/fleetspeak/src/client/config"
	fsdaemonservice "github.com/google/fleetspeak/fleetspeak/src/client/daemonservice"
	fsdpb "github.com/google/fleetspeak/fleetspeak/src/client/daemonservice/proto/fleetspeak_daemonservice"
	fsservice "github.com/google/fleetspeak/fleetspeak/src/client/service"
	fspb "github.com/google/fleetspeak/fleetspeak/src/common/proto/fleetspeak"
	"google.golang.org/protobuf/types/known/anypb"
)

var (
	flagHello = flag.String("hello", "", "Path to the `hello` daemon binary")
)

type Communicator struct {
	ctx fscomms.Context
}

func (c *Communicator) Setup(ctx fscomms.Context) error {
	c.ctx = ctx

	return nil
}

func (c *Communicator) Start() error {
	return nil
}

func (c *Communicator) Stop() {
}

func (c *Communicator) GetFileIfModified(ctx context.Context, service, name string, modSince time.Time) (data io.ReadCloser, mod time.Time, err error) {
	return nil, time.Time{}, errors.ErrUnsupported
}

func TestHello(t *testing.T) {
	if *flagHello == "" {
		t.Skip("Path to the `hello` daemon not specified")
	}

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	comm := &Communicator{}

	cfg, err := anypb.New(&fsdpb.Config{
		Argv: []string{*flagHello},
	})
	if err != nil {
		t.Fatalf("failed to create daemon config: %v", err)
	}

	client, err := fsclient.New(fsconfig.Configuration{
		PersistenceHandler: fsconfig.NewNoopPersistenceHandler(),
		FixedServices: []*fspb.ClientServiceConfig{
			{
				Name:    "hello",
				Factory: "Daemon",
				Config:  cfg,
			},
		},
	}, fsclient.Components{
		ServiceFactories: map[string]fsservice.Factory{
			"Daemon": fsdaemonservice.Factory,
		},
		Communicator: comm,
	})
	if err != nil {
		t.Fatalf("failed to create client: %v", err)
	}
	defer client.Stop()

	if err := comm.ctx.ProcessContactData(ctx, &fspb.ContactData{
		Messages: []*fspb.Message{
			{
				Source: &fspb.Address{
					ServiceName: "greeter",
				},
				Destination: &fspb.Address{
					ClientId:    comm.ctx.CurrentID().Bytes(),
					ServiceName: "hello",
				},
				Data: &anypb.Any{
					Value: []byte("foobar"),
				},
			},
		},
	}, false); err != nil {
		t.Fatalf("failed to send message: %v", err)
	}

	for {
		select {
		case msgInfo := <-comm.ctx.Outbox():
			msgInfo.Ack()

			if msgInfo.M.Source.ServiceName == "system" {
				t.Logf("received system message: %q", msgInfo.M)
				continue
			}

			if msgInfo.M.Source.ServiceName != "hello" {
				t.Errorf("expected message from `hello`, got: %q", msgInfo.M.Source.ServiceName)
			}
			if msgInfo.M.Destination.ServiceName != "greeter" {
				t.Errorf("expected message to `greeter`, got: %q", msgInfo.M.Destination.ServiceName)
			}
			if !bytes.Equal(msgInfo.M.Data.Value, []byte("Hello, foobar!")) {
				t.Errorf("expected `Hello, foobar!`, got: %q", msgInfo.M.Data)
			}

			return
		case <-ctx.Done():
			t.Fatalf("failed to receive response message: %v", ctx.Err())
		}
	}
}
