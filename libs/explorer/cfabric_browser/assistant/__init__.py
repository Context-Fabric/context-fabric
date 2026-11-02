"""Searching in plain language: the tools a language model gets to turn a
request ("all cases of >LHJM/ in the BHSA") into a search template that the
browser shows and runs.

`tools.py` has the tools; the model reaches them through MCP (the user's own
AI app, with their own subscription). The results always come from the
browser's search, never from the model's text.
"""
