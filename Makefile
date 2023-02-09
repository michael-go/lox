build-golox:
	cd golox && go build

build-rslox:
	cd rslox && make build-release

test-origin: build-golox build-rslox
	./test/origin/test.sh


	
