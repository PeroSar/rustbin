package main

/*
#include <stdlib.h>
*/
import "C"

import (
	"slices"
	"strings"
	"unsafe"

	enry "github.com/go-enry/go-enry/v2"
	"github.com/go-enry/go-enry/v2/data"
)

var classifierCandidates = slices.Collect(languageCandidates())

func languageCandidates() func(func(string) bool) {
	return func(yield func(string) bool) {
		for language := range data.LanguagesLogProbabilities {
			if !yield(language) {
				return
			}
		}
	}
}

//export DetectLanguageByClassifier
func DetectLanguageByClassifier(content *C.char, contentLen C.int) *C.char {
	if content == nil || contentLen <= 0 {
		return nil
	}

	language, _ := enry.GetLanguageByClassifier(
		C.GoBytes(unsafe.Pointer(content), contentLen),
		classifierCandidates,
	)
	if language == "" || language == "Other" {
		return nil
	}

	extensions := enry.GetLanguageExtensions(language)
	if len(extensions) == 0 {
		return nil
	}

	cleaned := make([]string, 0, len(extensions))
	for _, ext := range extensions {
		cleaned = append(cleaned, strings.TrimPrefix(ext, "."))
	}

	return C.CString(strings.Join(cleaned, "\n"))
}

//export FreeEnryString
func FreeEnryString(value *C.char) {
	if value == nil {
		return
	}

	C.free(unsafe.Pointer(value))
}

func main() {}
