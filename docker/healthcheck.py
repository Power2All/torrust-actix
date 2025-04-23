#!/usr/bin/python

import tomllib
import sys
from socket import *
from urllib.parse import urlparse
from http.client import HTTPConnection, HTTPSConnection
from datetime import datetime
from enum import Enum

class messageType(Enum):
    INFO = "I"
    ERROR = "E"
    WARN = "W"
    DEBUG = "D"

def consoleLog(messageType, message):
    print("%s [%s] %s" % (messageType.value, datetime.now().strftime("%Y-%m-%d %H:%M:%S"), message))

##############################################
# Get the DATA from the config file to parse #
##############################################

# Load the TOML configuration
with open("./target/release/config.toml", "rb") as f:
    data = tomllib.load(f)

api_binds = []
# Get the Enabled API blocks
for block in data['api_server']:
    if block['enabled'] == True:
        api_binds.append({ "ip": block['bind_address'].split(':')[0], "port": block['bind_address'].split(':')[1], "ssl": block['ssl'] })

http_binds = []
# Get the Enabled TCP blocks
for block in data['http_server']:
    if block['enabled'] == True:
        http_binds.append({ "ip": block['bind_address'].split(':')[0], "port": block['bind_address'].split(':')[1], "ssl": block['ssl'] })

udp_binds = []
# Get the Enabled UDP blocks
for block in data['udp_server']:
    if block['enabled'] == True:
        udp_binds.append({ "ip": block['bind_address'].split(':')[0], "port": block['bind_address'].split(':')[1] })

#####################################
# Check if the ports are accessible #
#####################################

ERROR_FOUND = False

# Validate API
for api_bind in api_binds:
    consoleLog(messageType.INFO, "Checking API(S) binding %s:%s SSL=%s" % (api_bind['ip'], api_bind['port'], api_bind['ssl']))
    if api_bind['ssl']:
        try:
            HTTPS_URL = f'https://{api_bind['ip']}:{api_bind['port']}'
            HTTPS_URL = urlparse(HTTPS_URL)
            connection = HTTPSConnection(HTTPS_URL.netloc, timeout=2)
            connection.request('HEAD', HTTPS_URL.path)
            if connection.getresponse():
                consoleLog(messageType.INFO, "Connection is available")
            else:
                ERROR_FOUND = True
                consoleLog(messageType.ERROR, "Connection is unavailable")
        except:
            ERROR_FOUND = True
            consoleLog(messageType.ERROR, "Connection is unavailable")

# Validate TCP
for http_bind in http_binds:
    consoleLog(messageType.INFO, "Checking HTTP(S) binding %s:%s SSL=%s" % (http_bind['ip'], http_bind['port'], http_bind['ssl']))
    if http_bind['ssl']:
        try:
            HTTPS_URL = f'https://{http_bind['ip']}:{http_bind['port']}'
            HTTPS_URL = urlparse(HTTPS_URL)
            connection = HTTPSConnection(HTTPS_URL.netloc, timeout=2)
            connection.request('HEAD', HTTPS_URL.path)
            if connection.getresponse():
                consoleLog(messageType.INFO, "Connection is available")
            else:
                ERROR_FOUND = True
                consoleLog(messageType.ERROR, "Connection is unavailable")
        except:
            ERROR_FOUND = True
            consoleLog(messageType.ERROR, "Connection is unavailable")

# Validate UDP
for udp_bind in udp_binds:
    consoleLog(messageType.INFO, "Checking UDP binding %s:%s" % (udp_bind['ip'], udp_bind['port']))
    try:
        s = socket(AF_INET, SOCK_DGRAM)
        code = s.connect_ex((udp_bind['ip'], udp_bind['port']))
        if code != 0:
            ERROR_FOUND = True
            consoleLog(messageType.ERROR, "Connection is unavailable")
        s.close()
        consoleLog(messageType.INFO, "Connection is available")
    except:
        ERROR_FOUND = True
        consoleLog(messageType.ERROR, "Connection is unavailable")

if ERROR_FOUND:
    consoleLog(messageType.ERROR, "Exit Code 1")
    sys.exit(1)
else:
    consoleLog(messageType.INFO, "Exit Code 0")
    sys.exit(0)