module tests

go 1.26

require (
	github.com/google/fleetspeak v0.0.0
	google.golang.org/protobuf v1.34.1
)

require (
	github.com/go-ole/go-ole v1.3.0 // indirect
	github.com/golang/glog v1.2.4 // indirect
	github.com/shirou/gopsutil v3.21.11+incompatible // indirect
	github.com/stretchr/testify v1.12.1 // indirect
	github.com/tklauser/go-sysconf v0.3.13 // indirect
	github.com/tklauser/numcpus v0.7.0 // indirect
	github.com/yusufpapurcu/wmi v1.2.4 // indirect
	golang.org/x/sync v0.7.0 // indirect
	golang.org/x/sys v0.19.0 // indirect
)

// TODO(@panhania): Move `vendor` folder to the root directory.
replace github.com/google/fleetspeak => ../crates/fleetspeak-proto/vendor/fleetspeak
