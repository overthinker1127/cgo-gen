package main

import (
	"fmt"
	"log"

	"simplegostruct/pkg/demo"
)

func main() {
	api, err := demo.NewThingApi()
	if err != nil {
		log.Fatal(err)
	}
	defer api.Close()

	item, err := demo.NewThingModel()
	if err != nil {
		log.Fatal(err)
	}
	defer item.Close()

	item.SetName("seed-from-go")
	item.SetValue(99)

	if !api.SelectThing(1, item) {
		log.Fatal("select failed")
	}
	fmt.Printf("after select: name=%s value=%d\n", item.GetName(), item.GetValue())

	pos := int32(0)
	if !api.NextThing(&pos, item) {
		log.Fatal("next failed")
	}
	item.SetName("edited-from-go")
	item.SetValue(item.GetValue() + 1)

	fmt.Printf("after next: pos=%d name=%s value=%d\n", pos, item.GetName(), item.GetValue())
}
