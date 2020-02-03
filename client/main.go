package main

import (
	"flag"
	"fmt"
	"time"

	mqtt "github.com/eclipse/paho.mqtt.golang"
	"github.com/google/uuid"
)

func newClient(id int) {
	opts := mqtt.NewClientOptions().
		AddBroker(fmt.Sprintf("tcp://%s", *addr)).
		SetClientID(fmt.Sprintf("%v-%s", id, uuid.New().String())).
		SetKeepAlive(58 * time.Second).
		SetProtocolVersion(4).
		SetCleanSession(true).
		SetPingTimeout(58 * time.Second).
		SetMaxReconnectInterval(time.Second).
		SetConnectTimeout(15 * time.Second).
		SetUsername("uname").
		SetPassword("passwd").
		SetAutoReconnect(true).
		SetWriteTimeout(15 * time.Second).
		SetConnectionLostHandler(func(c mqtt.Client, err error) {
			fmt.Println("onConnectLost", err)
		}).
		SetDefaultPublishHandler(func(c mqtt.Client, msg mqtt.Message) {
			fmt.Println("onPublish", msg)
		})
	c := mqtt.NewClient(opts)
	for {
		if token := c.Connect(); token.Wait() && token.Error() != nil {
			fmt.Println(token.Error())
			time.Sleep(time.Second)
			continue
		}
		break
	}

	for {
		if token := c.Subscribe("$sys/broadcast", 1, func(c mqtt.Client, msg mqtt.Message) {
			// fmt.Printf("onPublish %+v\n", msg)
		}); token.Wait() && token.Error() != nil {
			fmt.Println(token.Error())
			time.Sleep(time.Second)
			continue
		}
		break
	}

	// for {
	// 	if token := c.Publish("/aaa/bbb", 1, false, "payload"); token.Wait() && token.Error() != nil {
	// 		fmt.Println(token.Error())
	// 		time.Sleep(time.Second)
	// 		continue
	// 	}
	// 	time.Sleep(55 * time.Second)
	// }
}

var n = flag.Int("n", 100, "")
var addr = flag.String("a", "127.0.0.1:6315", "")

func main() {
	flag.Parse()
	for i := 0; i < *n; i++ {
		go newClient(i)
	}

	// Publishes to the server by sending the message
	select {}
}
