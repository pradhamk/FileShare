#!/bin/bash

cargo build --release

sudo rm /usr/bin/fsclient

sudo ln -s [PATH TO CLIENT DIR]/fsclient /usr/bin/fsclient
