#import "@preview/ilm:1.4.2": *

#set text(lang: "en")

#show: ilm.with(
  title: [Sonos Coding Challenge Write-up],
  author: "Yassine Safraoui",
  date: none,
  abstract: [],
  preface: none,
  bibliography: none,
  figure-index: (enabled: true),
  table-index: (enabled: true),
  listing-index: (enabled: true),
)
#include "content/introduction.typ"
#include "content/communication-protocol.typ"
#include "content/tcp-communication.typ"
#include "content/handling-audio.typ"
#include "content/cli.typ"
#include "content/future-work.typ"
#include "content/ai-usage.typ"
