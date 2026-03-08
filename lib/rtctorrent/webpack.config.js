const path = require('path');
const NodePolyfillPlugin = require('node-polyfill-webpack-plugin');

module.exports = [
  {
    name: 'browser',
    entry: './src/rtctorrent.js',
    output: {
      filename: 'rtctorrent.browser.js',
      path: path.resolve(__dirname, 'dist'),
      library: {
        name: 'RtcTorrent',
        type: 'umd',
        export: 'default'
      },
      globalObject: 'this'
    },
    target: 'web',
    module: {
      rules: [
        {
          test: /\.js$/,
          exclude: /node_modules/,
          use: {
            loader: 'babel-loader',
            options: {
              presets: ['@babel/preset-env']
            }
          }
        }
      ]
    },
    resolve: {
      extensions: ['.js'],
      fallback: {
        "fs": false,
        "path": require.resolve("path-browserify"),
        "crypto": require.resolve("crypto-browserify"),
        "stream": require.resolve("stream-browserify"),
        "buffer": require.resolve("buffer/"),
        "process": require.resolve("process/browser")
      }
    },
    plugins: [
      new NodePolyfillPlugin()
    ],
    mode: 'production',
    devtool: 'source-map'
  },
  {
    name: 'node',
    entry: './src/rtctorrent.js',
    output: {
      filename: 'rtctorrent.node.js',
      path: path.resolve(__dirname, 'dist'),
      library: {
        name: 'RtcTorrent',
        type: 'umd',
        export: 'default'
      },
      globalObject: 'this'
    },
    target: 'node',
    externals: {
      'wrtc': 'commonjs wrtc',
      '@roamhq/wrtc': 'commonjs @roamhq/wrtc',
      'node-webrtc': 'commonjs node-webrtc'
    },
    module: {
      rules: [
        {
          test: /\.js$/,
          exclude: /node_modules/,
          use: {
            loader: 'babel-loader',
            options: {
              presets: ['@babel/preset-env']
            }
          }
        }
      ]
    },
    mode: 'production',
    devtool: 'source-map'
  }
];