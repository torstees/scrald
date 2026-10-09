# Images

A missing local image: ![The fjord](images/fjord.png "Fjord at dawn")

Spaces are percent-decoded: ![](my%20pictures/boat.png)

A remote image, blocked by default: ![Logo](https://example.com/logo.png)

A data image: ![dot](data:image/gif;base64,R0lGODlhAQABAAAAACw=)

An unsupported scheme: ![x](javascript:alert(1))

Raw HTML images never load files: <img src="secret.png"> <img src="https://example.com/t.png">
