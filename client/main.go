package main

import (
	"flag"
	"fmt"
	"time"

	mqtt "github.com/eclipse/paho.mqtt.golang"
)

func newClient(id int) {
	opts := mqtt.NewClientOptions().
		AddBroker("tcp://127.0.0.1:1883").
		SetClientID(fmt.Sprintf("%v", id)).
		SetKeepAlive(13 * time.Second).
		SetProtocolVersion(4).
		SetCleanSession(true).
		SetPingTimeout(10 * time.Second).
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
		}
		time.Sleep(time.Second)
	}
}

var n = flag.Int("n", 100, "")

func main() {
	flag.Parse()
	for i := 0; i < *n; i++ {
		time.Sleep(5 * time.Millisecond)
		go newClient(i)
	}

	// Publishes to the server by sending the message
	select {}
}
