//go:build ignore

// Tags prints the -tags that omit every Tailscale feature tailcat doesn't need, as tailcat's own
// wasm build does (its internal/buildtags), to make tailgate.wasm smaller. mksite.sh runs it.
package main

import (
	"fmt"
	"slices"
	"strings"

	"tailscale.com/feature/featuretags"
)

func main() {
	keep := featuretags.Requires("netstack")
	var tags []string
	for ft := range featuretags.Features {
		if ft != "" && ft.IsOmittable() {
			if _, ok := keep[ft]; ok {
				continue
			}
			tags = append(tags, ft.OmitTag())
		}
	}
	slices.Sort(tags)
	fmt.Println(strings.Join(tags, ","))
}
