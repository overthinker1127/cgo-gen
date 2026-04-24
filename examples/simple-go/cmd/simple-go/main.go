package main

import (
	"fmt"
	"log"

	"simplego/pkg/foo"
)

func main() {
	bar := foo.NewBar(7)
	defer bar.Close()

	bar.SetValue(9)

	name, err := bar.Name()
	if err != nil {
		log.Fatal(err)
	}

	fmt.Printf("name=%s value=%d add=%d\n", name, bar.Value(), foo.Add(3, 4))
}
