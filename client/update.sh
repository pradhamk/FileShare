#!/bin/bash

cargo build --release

sudo rm /usr/local/bin/fsclient

sudo ln -s [PATH TO CLIENT DIR]/fsclient /usr/local/bin/fsclient
