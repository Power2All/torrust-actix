#!/usr/bin/python

import tomllib
import sys
import re
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

def check_udp_port(host, port):
    try:
        s = socket(AF_INET, SOCK_DGRAM)
        code = s.bind((host, port))
        s.close()
        return False
    except:
        return True

def consoleLog(messageType, message):
    print("%s [%s] %s" % (messageType.value, datetime.now().strftime("%Y-%m-%d %H:%M:%S"), message))

##############################################
# Get the DATA from the config file to parse #
##############################################

# Load the TOML configuration
with open("./target/release/config.toml", "rb") as f:
    data = tomllib.load(f)

# Get the Enabled API blocks
api_binds = []
for block in data['api_server']:
    if block['enabled'] == True:
        ipv4_search = re.search(r"((?:(?:25[0-5]|(?:2[0-4]|1\d|[1-9]|)\d)\.?\b){4})\:([0-9]+)", block['bind_address'])
        ipv6_search = re.search(r"\[(.+)\]\:([0-9]+)", block['bind_address'])
        if ipv4_search != None:
            if ipv4_search[1] == "0.0.0.0":
                api_binds.append({ "ip": "127.0.0.1", "port": int(ipv4_search[2]), "ssl": block['ssl'] })
            else:
                api_binds.append({ "ip": ipv4_search[1], "port": int(ipv4_search[2]), "ssl": block['ssl'] })
        if ipv6_search != None:
            if ipv6_search[1] == "::":
                api_binds.append({ "ip": "::1", "port": int(ipv6_search[2]), "ssl": block['ssl'] })
            else:
                api_binds.append({ "ip": f"[{ipv6_search[1]}]", "port": int(ipv6_search[2]), "ssl": block['ssl'] })

# Get the Enabled TCP blocks
http_binds = []
for block in data['http_server']:
    if block['enabled'] == True:
        ipv4_search = re.search(r"((?:(?:25[0-5]|(?:2[0-4]|1\d|[1-9]|)\d)\.?\b){4})\:([0-9]+)", block['bind_address'])
        ipv6_search = re.search(r"\[(.+)\]\:([0-9]+)", block['bind_address'])
        if ipv4_search != None:
            if ipv4_search[1] == "0.0.0.0":
                http_binds.append({ "ip": "127.0.0.1", "port": int(ipv4_search[2]), "ssl": block['ssl'] })
            else:
                http_binds.append({ "ip": ipv4_search[1], "port": int(ipv4_search[2]), "ssl": block['ssl'] })
        if ipv6_search != None:
            if ipv6_search[1] == "::":
                http_binds.append({ "ip": "::1", "port": int(ipv6_search[2]), "ssl": block['ssl'] })
            else:
                http_binds.append({ "ip": f"[{ipv6_search[1]}]", "port": int(ipv6_search[2]), "ssl": block['ssl'] })

# Get the Enabled UDP blocks
udp_binds = []
for block in data['udp_server']:
    if block['enabled'] == True:
        ipv4_search = re.search(r"((?:(?:25[0-5]|(?:2[0-4]|1\d|[1-9]|)\d)\.?\b){4})\:([0-9]+)", block['bind_address'])
        ipv6_search = re.search(r"\[(.+)\]\:([0-9]+)", block['bind_address'])
        if ipv4_search != None:
            udp_binds.append({ "ip": ipv4_search[1], "port": int(ipv4_search[2]) })
        if ipv6_search != None:
            udp_binds.append({ "ip": f"[{ipv6_search[1]}]", "port": int(ipv6_search[2]) })

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
    else:
        try:
            HTTP_URL = f'http://{api_bind['ip']}:{api_bind['port']}'
            HTTP_URL = urlparse(HTTP_URL)
            connection = HTTPConnection(HTTP_URL.netloc, timeout=2)
            connection.request('HEAD', HTTP_URL.path)
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
    else:
        try:
            HTTP_URL = f'http://{http_bind['ip']}:{http_bind['port']}'
            HTTP_URL = urlparse(HTTP_URL)
            connection = HTTPConnection(HTTP_URL.netloc, timeout=2)
            connection.request('HEAD', HTTP_URL.path)
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
#     try:
    if check_udp_port(udp_bind['ip'], int(udp_bind['port'])):
        consoleLog(messageType.INFO, "Connection is available")
    else:
        ERROR_FOUND = True
        consoleLog(messageType.ERROR, "Connection is unavailable")
#     except:
#         ERROR_FOUND = True
#         consoleLog(messageType.ERROR, "Connection is unavailable")

if ERROR_FOUND:
    consoleLog(messageType.ERROR, "Exit Code 1")
    sys.exit(1)
else:
    consoleLog(messageType.INFO, "Exit Code 0")
    sys.exit(0)